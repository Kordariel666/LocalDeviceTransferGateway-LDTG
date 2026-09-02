use super::state::{
    AuthDecision, CompletedUpload, DirectoryEntry, DirectoryListing, DirectoryPage, DownloadLease,
    SessionCreateError, SessionRecord, TransferServiceState, UploadChunkLease, UploadRecord,
    ACCESS_CODE_DIGITS, CHUNK_SIZE, DISK_RESERVE, MAX_UPLOADS_PER_ADDRESS,
};
use super::ConnectionSecurity;
use crate::domain::{
    network::same_subnet,
    shares::{
        create_upload_partial, delete_open_upload, is_hidden_or_managed, publish_open_upload,
        safe_existing, safe_file_name_for_root,
    },
};
use axum::{
    body::{Body, Bytes},
    extract::{
        rejection::{BytesRejection, JsonRejection, PathRejection, QueryRejection},
        ConnectInfo, DefaultBodyLimit, Path as AxumPath, Query, State,
    },
    http::{header, HeaderMap, HeaderValue, Method, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    fs,
    io::{Seek, Write},
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(RustEmbed)]
#[folder = "../apps/mobile/dist/"]
struct MobileAssets;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorBody {
    code: String,
    message: String,
}

#[derive(Debug)]
struct ApiFailure {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiFailure {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiFailure {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorBody {
                code: self.code.into(),
                message: self.message,
            }),
        )
            .into_response()
    }
}

type ApiResult<T> = Result<T, ApiFailure>;

#[derive(Clone)]
struct UploadChunkPermit {
    _lease: Arc<UploadChunkLease>,
}

const REQUEST_TIMEOUT: Duration = Duration::from_secs(2 * 60);
const DIRECTORY_PAGE_SIZE: usize = 200;
const DIRECTORY_SCAN_LIMIT: usize = 256;

pub fn router(state: Arc<TransferServiceState>) -> Router {
    Router::new()
        .route(
            "/api/v1/auth",
            post(auth).layer(DefaultBodyLimit::max(1024)),
        )
        .route("/api/v1/session", get(session))
        .route("/api/v1/logout", post(logout))
        .route("/api/v1/downloads", get(list_downloads))
        .route("/api/v1/download", get(download).head(download_head))
        .route(
            "/api/v1/uploads",
            post(create_upload).layer(DefaultBodyLimit::max(32 * 1024)),
        )
        .route(
            "/api/v1/uploads/{id}",
            get(upload_status).patch(upload_chunk).delete(cancel_upload),
        )
        .route("/api/v1/uploads/{id}/complete", post(complete_upload))
        .method_not_allowed_fallback(method_not_allowed)
        .fallback(static_asset)
        .layer(DefaultBodyLimit::max(CHUNK_SIZE + 1024))
        .layer(middleware::from_fn_with_state(state.clone(), request_guard))
        .with_state(state)
}

async fn method_not_allowed() -> ApiFailure {
    ApiFailure::new(
        StatusCode::METHOD_NOT_ALLOWED,
        "METHOD_NOT_ALLOWED",
        "Diese Aktion wird von DMDC nicht angeboten.",
    )
}

async fn request_guard(
    State(state): State<Arc<TransferServiceState>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let Some(ConnectInfo(client)) = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .copied()
    else {
        return secure_response(
            ApiFailure::new(
                StatusCode::FORBIDDEN,
                "CLIENT_UNKNOWN",
                "Clientadresse konnte nicht geprüft werden.",
            )
            .into_response(),
        );
    };
    let client_v4 = match client.ip() {
        IpAddr::V4(value) => value,
        _ => {
            return secure_response(
                ApiFailure::new(
                    StatusCode::FORBIDDEN,
                    "IPV4_REQUIRED",
                    "Nur lokale IPv4-Verbindungen sind erlaubt.",
                )
                .into_response(),
            )
        }
    };
    if !same_subnet(client_v4, &state.interface) {
        return secure_response(
            ApiFailure::new(
                StatusCode::FORBIDDEN,
                "OUTSIDE_SUBNET",
                "Die Anfrage stammt nicht aus dem freigegebenen lokalen Subnetz.",
            )
            .into_response(),
        );
    }
    let expected_host = state.expected_host();
    let supplied_host = request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if supplied_host != expected_host {
        return secure_response(
            ApiFailure::new(
                StatusCode::MISDIRECTED_REQUEST,
                "HOST_REJECTED",
                "Ungültiger Host-Header.",
            )
            .into_response(),
        );
    }
    if matches!(
        *request.method(),
        Method::POST | Method::PATCH | Method::DELETE | Method::PUT
    ) {
        let expected_origin = format!("http://{expected_host}");
        let supplied_origin = request
            .headers()
            .get(header::ORIGIN)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("");
        if supplied_origin != expected_origin {
            return secure_response(
                ApiFailure::new(
                    StatusCode::FORBIDDEN,
                    "ORIGIN_REJECTED",
                    "Die Anfrage stammt nicht von der DMDC-Seite.",
                )
                .into_response(),
            );
        }
    }
    let address = client.ip();
    let authenticated = match cookie_token(request.headers()) {
        Some(token) => state.session_token_is_active(token, address).await,
        None => false,
    };
    if authenticated {
        if let Some(security) = request.extensions().get::<Arc<ConnectionSecurity>>() {
            if !security.mark_authenticated() {
                let mut response = secure_response(
                    ApiFailure::new(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "CONNECTION_CAPACITY",
                        "Es sind bereits zu viele angemeldete Verbindungen aktiv.",
                    )
                    .into_response(),
                );
                response
                    .headers_mut()
                    .insert(header::CONNECTION, HeaderValue::from_static("close"));
                return response;
            }
        }
    }
    let Some(_request_permit) = state.begin_request(address, authenticated).await else {
        return secure_response(
            ApiFailure::new(
                StatusCode::TOO_MANY_REQUESTS,
                "REQUEST_LIMIT",
                "Es laufen bereits zu viele Anfragen.",
            )
            .into_response(),
        );
    };
    if request.method() == Method::PATCH && request.uri().path().starts_with("/api/v1/uploads/") {
        let session = match authorized(&state, request.headers(), address, true).await {
            Ok(session) => session,
            Err(error) => return secure_response(error.into_response()),
        };
        let Some(id) = request
            .uri()
            .path()
            .strip_prefix("/api/v1/uploads/")
            .filter(|id| !id.is_empty() && !id.contains('/'))
        else {
            return secure_response(
                ApiFailure::new(
                    StatusCode::BAD_REQUEST,
                    "UPLOAD_ID_INVALID",
                    "Die Upload-ID ist ungültig.",
                )
                .into_response(),
            );
        };
        let record = match owned_upload(&state, id, &session).await {
            Ok(record) => record,
            Err(error) => return secure_response(error.into_response()),
        };
        let lease = {
            let record = record.lock().await;
            if record.finalizing {
                return secure_response(
                    ApiFailure::new(
                        StatusCode::CONFLICT,
                        "UPLOAD_FINALIZING",
                        "Die Datei wird bereits endgültig übernommen.",
                    )
                    .into_response(),
                );
            }
            match state.begin_upload_chunk(&record) {
                Ok(lease) => lease,
                Err(scope) => {
                    let message = if scope == "upload" {
                        "Für diese Upload-ID wird bereits ein Block verarbeitet."
                    } else {
                        "DMDC verarbeitet bereits die maximale Zahl gleichzeitiger Uploadblöcke."
                    };
                    return secure_response(
                        ApiFailure::new(
                            StatusCode::TOO_MANY_REQUESTS,
                            "UPLOAD_CHUNK_BUSY",
                            message,
                        )
                        .into_response(),
                    );
                }
            }
        };
        request.extensions_mut().insert(UploadChunkPermit {
            _lease: Arc::new(lease),
        });
    }
    let waits_for_commit = request.method() == Method::POST
        && request.uri().path().starts_with("/api/v1/uploads/")
        && request.uri().path().ends_with("/complete");
    let response = if waits_for_commit {
        next.run(request).await
    } else {
        match tokio::time::timeout(REQUEST_TIMEOUT, next.run(request)).await {
            Ok(response) => response,
            Err(_) => ApiFailure::new(
                StatusCode::REQUEST_TIMEOUT,
                "REQUEST_TIMEOUT",
                "Die Anfrage hat das zulässige Zeitlimit überschritten.",
            )
            .into_response(),
        }
    };
    let mut response = secure_response(response);
    if !authenticated {
        response
            .headers_mut()
            .insert(header::CONNECTION, HeaderValue::from_static("close"));
    }
    response
}

fn secure_response(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert(
        "cross-origin-opener-policy",
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        "cross-origin-resource-policy",
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        "x-permitted-cross-domain-policies",
        HeaderValue::from_static("none"),
    );
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    headers.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self' data: blob:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'"));
    response
}

fn client_ip(connect: ConnectInfo<SocketAddr>) -> IpAddr {
    connect.0.ip()
}

fn cookie_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix("dmdc_session="))
}

async fn authorized(
    state: &TransferServiceState,
    headers: &HeaderMap,
    client: IpAddr,
    csrf: bool,
) -> ApiResult<SessionRecord> {
    let token = cookie_token(headers).ok_or_else(|| {
        ApiFailure::new(
            StatusCode::UNAUTHORIZED,
            "AUTH_REQUIRED",
            "Bitte erneut mit dem Zugangscode anmelden.",
        )
    })?;
    let session = state.authenticate(token, client).await.ok_or_else(|| {
        ApiFailure::new(
            StatusCode::UNAUTHORIZED,
            "SESSION_INVALID",
            "Die Sitzung ist nicht mehr gültig.",
        )
    })?;
    if csrf {
        let supplied = headers
            .get("x-dmdc-csrf")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("");
        if supplied
            .as_bytes()
            .ct_eq(session.csrf.as_bytes())
            .unwrap_u8()
            != 1
        {
            return Err(ApiFailure::new(
                StatusCode::FORBIDDEN,
                "CSRF_INVALID",
                "Sicherheitsnachweis der Sitzung fehlt.",
            ));
        }
    }
    Ok(session)
}

async fn ensure_session_active(
    state: &TransferServiceState,
    session: &SessionRecord,
) -> ApiResult<()> {
    if state.session_is_active(session).await {
        Ok(())
    } else {
        Err(ApiFailure::new(
            StatusCode::UNAUTHORIZED,
            "SESSION_INVALID",
            "Die Sitzung ist nicht mehr gültig.",
        ))
    }
}

#[derive(Deserialize)]
struct AuthRequest {
    code: String,
}

async fn auth(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    payload: Result<Json<AuthRequest>, JsonRejection>,
) -> ApiResult<Response> {
    let Json(payload) = payload.map_err(|error| {
        ApiFailure::new(
            error.status(),
            "AUTH_BODY_INVALID",
            "Der Zugangscode konnte nicht gelesen werden.",
        )
    })?;
    let address = client.ip();
    match state.verify_access_code(address, &payload.code) {
        AuthDecision::Accepted => {}
        AuthDecision::Invalid => {
            return Err(ApiFailure::new(
                StatusCode::UNAUTHORIZED,
                "CODE_INVALID",
                format!("Der {ACCESS_CODE_DIGITS}-stellige Zugangscode ist nicht richtig."),
            ));
        }
        AuthDecision::AddressBlocked => {
            return Err(ApiFailure::new(
                StatusCode::TOO_MANY_REQUESTS,
                "CODE_BLOCKED",
                "Zu viele falsche Versuche. Bitte in fünf Minuten erneut probieren.",
            ));
        }
        AuthDecision::GlobalBlocked => {
            return Err(ApiFailure::new(
                StatusCode::TOO_MANY_REQUESTS,
                "SERVICE_CODE_BLOCKED",
                "Der dienstweite Schutz wurde ausgelöst. Bitte in fünf Minuten erneut probieren.",
            ));
        }
    }
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("Mobiler Browser")
        .chars()
        .take(180)
        .collect();
    let session = state
        .create_session(address, user_agent)
        .await
        .map_err(|limit| match limit {
            SessionCreateError::AddressLimit => ApiFailure::new(
                StatusCode::TOO_MANY_REQUESTS,
                "SESSION_CLIENT_LIMIT",
                "Für diese Geräteadresse bestehen bereits zu viele Sitzungen.",
            ),
            SessionCreateError::GlobalLimit => ApiFailure::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "SESSION_CAPACITY",
                "Der Sitzungspool ist belegt. Bestehende Sitzungen bleiben erhalten.",
            ),
        })?;
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&format!(
            "dmdc_session={}; Path=/; HttpOnly; SameSite=Strict",
            session.token
        ))
        .expect("generated cookie is valid"),
    );
    Ok(response)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionResponse {
    service_id: String,
    csrf_token: String,
    download_enabled: bool,
    upload_enabled: bool,
    max_upload_bytes: Option<u64>,
}

async fn session(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> ApiResult<Json<SessionResponse>> {
    let session = authorized(&state, &headers, client_ip(ConnectInfo(client)), false).await?;
    Ok(Json(SessionResponse {
        service_id: state.service_id.clone(),
        csrf_token: session.csrf,
        download_enabled: state.roots.download.is_some(),
        upload_enabled: state.roots.upload.is_some(),
        max_upload_bytes: state.settings.max_upload_bytes,
    }))
}

async fn logout(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let session = authorized(&state, &headers, client.ip(), true).await?;
    state.revoke_session(&session.id).await;
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static("dmdc_session=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0"),
    );
    Ok(response)
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(default)]
    path: String,
    cursor: Option<String>,
    page: Option<u64>,
    q: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DirectoryResponse {
    path: String,
    query: String,
    entries: Vec<DirectoryEntry>,
    next_cursor: Option<String>,
    next_page: Option<u64>,
}

fn relative_url_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .filter_map(|part| match part {
            std::path::Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn natural_compare(left: &str, right: &str) -> Ordering {
    let left = left.to_lowercase();
    let right = right.to_lowercase();
    let left = left.as_bytes();
    let right = right.as_bytes();
    let (mut left_index, mut right_index) = (0, 0);
    while left_index < left.len() && right_index < right.len() {
        if left[left_index].is_ascii_digit() && right[right_index].is_ascii_digit() {
            let left_end = (left_index..left.len())
                .find(|index| !left[*index].is_ascii_digit())
                .unwrap_or(left.len());
            let right_end = (right_index..right.len())
                .find(|index| !right[*index].is_ascii_digit())
                .unwrap_or(right.len());
            let left_significant = (left_index..left_end)
                .find(|index| left[*index] != b'0')
                .unwrap_or(left_end.saturating_sub(1));
            let right_significant = (right_index..right_end)
                .find(|index| right[*index] != b'0')
                .unwrap_or(right_end.saturating_sub(1));
            let length_order = (left_end - left_significant).cmp(&(right_end - right_significant));
            if length_order != Ordering::Equal {
                return length_order;
            }
            let number_order =
                left[left_significant..left_end].cmp(&right[right_significant..right_end]);
            if number_order != Ordering::Equal {
                return number_order;
            }
            let zero_order = (left_end - left_index).cmp(&(right_end - right_index));
            if zero_order != Ordering::Equal {
                return zero_order;
            }
            left_index = left_end;
            right_index = right_end;
        } else {
            let order = left[left_index].cmp(&right[right_index]);
            if order != Ordering::Equal {
                return order;
            }
            left_index += 1;
            right_index += 1;
        }
    }
    left.len().cmp(&right.len())
}

fn read_directory_page(
    listing: &DirectoryListing,
    requested_page: u64,
) -> ApiResult<DirectoryPage> {
    let mut cursor = listing.cursor.lock().map_err(|_| {
        ApiFailure::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DIRECTORY_CURSOR_FAILED",
            "Der Ordnercursor ist nicht mehr verfügbar.",
        )
    })?;
    if cursor.last_activity.elapsed() >= super::state::DIRECTORY_CURSOR_TTL {
        return Err(ApiFailure::new(
            StatusCode::BAD_REQUEST,
            "CURSOR_INVALID",
            "Der Ordnercursor ist abgelaufen.",
        ));
    }
    if let Some(page) = cursor
        .last_page
        .as_ref()
        .filter(|page| page.sequence == requested_page)
        .cloned()
    {
        cursor.last_activity = Instant::now();
        return Ok(page);
    }
    if requested_page != cursor.next_page || cursor.exhausted {
        return Err(ApiFailure::new(
            StatusCode::BAD_REQUEST,
            "CURSOR_PAGE_INVALID",
            "Die angeforderte Ordnerseite passt nicht zum Cursor.",
        ));
    }
    let mut entries = Vec::new();
    for _ in 0..DIRECTORY_SCAN_LIMIT {
        let Some(item) = cursor.iterator.next() else {
            cursor.exhausted = true;
            break;
        };
        let Ok(item) = item else {
            continue;
        };
        let requested_path = Path::new(&listing.path).join(item.file_name());
        let relative_path = relative_url_path(&listing.root, &listing.root.join(requested_path));
        let Ok(checked_path) = safe_existing(&listing.root, &relative_path, None) else {
            continue;
        };
        let Ok((resolved, _file, metadata)) = listing.root_anchor.open_existing(&checked_path)
        else {
            continue;
        };
        if is_hidden_or_managed(&resolved, &metadata) {
            continue;
        }
        let relative_path = relative_url_path(&listing.root, &resolved);
        let name = resolved
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| item.file_name().to_string_lossy().into_owned());
        if !listing.filter.is_empty() && !name.to_lowercase().contains(&listing.filter) {
            continue;
        }
        let modified_at = metadata
            .modified()
            .ok()
            .map(|value| DateTime::<Utc>::from(value).to_rfc3339());
        entries.push(DirectoryEntry {
            name,
            path: relative_path,
            kind: if metadata.is_dir() {
                "directory"
            } else {
                "file"
            }
            .into(),
            size: if metadata.is_file() {
                metadata.len()
            } else {
                0
            },
            modified_at,
        });
        if entries.len() >= DIRECTORY_PAGE_SIZE {
            break;
        }
    }
    entries.sort_by(|left, right| {
        (left.kind != "directory")
            .cmp(&(right.kind != "directory"))
            .then_with(|| natural_compare(&left.name, &right.name))
            .then_with(|| left.name.cmp(&right.name))
    });
    cursor.last_activity = Instant::now();
    let page = DirectoryPage {
        sequence: requested_page,
        entries,
        exhausted: cursor.exhausted,
    };
    cursor.next_page = cursor.next_page.saturating_add(1);
    cursor.last_page = Some(page.clone());
    Ok(page)
}

async fn list_downloads(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    query: Result<Query<ListQuery>, QueryRejection>,
) -> ApiResult<Json<DirectoryResponse>> {
    let Query(query) = query.map_err(|error| {
        ApiFailure::new(
            error.status(),
            "QUERY_INVALID",
            "Die Ordneranfrage ist ungültig.",
        )
    })?;
    let session = authorized(&state, &headers, client.ip(), false).await?;
    let root = state.roots.download.as_ref().cloned().ok_or_else(|| {
        ApiFailure::new(
            StatusCode::NOT_FOUND,
            "DOWNLOAD_DISABLED",
            "Die Downloadfreigabe ist nicht aktiv.",
        )
    })?;
    let filter = query.q.unwrap_or_default().to_lowercase();
    let requested_page = match (&query.cursor, query.page) {
        (Some(_), Some(page)) => page,
        (Some(_), None) => {
            return Err(ApiFailure::new(
                StatusCode::BAD_REQUEST,
                "CURSOR_PAGE_REQUIRED",
                "Für einen Ordnercursor muss die Seitennummer mitgesendet werden.",
            ))
        }
        (None, Some(page)) if page != 0 => {
            return Err(ApiFailure::new(
                StatusCode::BAD_REQUEST,
                "CURSOR_PAGE_INVALID",
                "Eine neue Ordnerabfrage beginnt mit Seite null.",
            ))
        }
        _ => 0,
    };
    let (cursor_id, listing, created) = if let Some(cursor_id) = query.cursor {
        let listing = state
            .directory_listing(&cursor_id, &session.id, &query.path, &filter)
            .await
            .ok_or_else(|| {
                ApiFailure::new(
                    StatusCode::BAD_REQUEST,
                    "CURSOR_INVALID",
                    "Der Ordnercursor ist abgelaufen oder gehört zu einer anderen Anfrage.",
                )
            })?;
        (cursor_id, listing, false)
    } else {
        let open_permit = state
            .begin_listing(&session.id, session.address)
            .await
            .ok_or_else(|| {
                ApiFailure::new(
                    StatusCode::TOO_MANY_REQUESTS,
                    "DIRECTORY_LIMIT",
                    "Es werden bereits zu viele Ordnerseiten vorbereitet.",
                )
            })?;
        let open_root = root.clone();
        let open_path = query.path.clone();
        let iterator = tokio::task::spawn_blocking(move || {
            let _permit = open_permit;
            let folder = safe_existing(&open_root, &open_path, Some(true)).map_err(|_| {
                ApiFailure::new(
                    StatusCode::NOT_FOUND,
                    "PATH_UNAVAILABLE",
                    "Ordner ist nicht verfügbar.",
                )
            })?;
            fs::read_dir(folder).map_err(|_| {
                ApiFailure::new(
                    StatusCode::NOT_FOUND,
                    "PATH_UNAVAILABLE",
                    "Ordner ist nicht verfügbar.",
                )
            })
        })
        .await
        .map_err(|_| {
            ApiFailure::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DIRECTORY_TASK_FAILED",
                "Der Ordner konnte nicht sicher vorbereitet werden.",
            )
        })??;
        let (cursor_id, listing) = state
            .create_directory_listing(&session, query.path.clone(), filter.clone(), root, iterator)
            .await
            .map_err(|scope| {
                if scope == "invalid" {
                    return ApiFailure::new(
                        StatusCode::UNAUTHORIZED,
                        "SESSION_INVALID",
                        "Die Sitzung ist nicht mehr gültig.",
                    );
                }
                let message = match scope {
                    "session" => "Diese Sitzung hat bereits zu viele offene Ordnercursor.",
                    "address" => {
                        "Von dieser Geräteadresse bestehen bereits zu viele offene Ordnercursor."
                    }
                    _ => "Es bestehen bereits zu viele offene Ordnercursor.",
                };
                ApiFailure::new(
                    StatusCode::TOO_MANY_REQUESTS,
                    "DIRECTORY_CURSOR_LIMIT",
                    message,
                )
            })?;
        (cursor_id, listing, true)
    };
    let page_permit = match state.begin_listing(&session.id, session.address).await {
        Some(permit) => permit,
        None => {
            if created {
                state.remove_directory_listing(&cursor_id).await;
            }
            return Err(ApiFailure::new(
                StatusCode::TOO_MANY_REQUESTS,
                "DIRECTORY_LIMIT",
                "Es werden bereits zu viele Ordnerseiten vorbereitet.",
            ));
        }
    };
    let page_listing = listing.clone();
    let page = tokio::task::spawn_blocking(move || {
        let _permit = page_permit;
        read_directory_page(&page_listing, requested_page)
    })
    .await
    .map_err(|_| {
        ApiFailure::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DIRECTORY_TASK_FAILED",
            "Die Ordnerseite konnte nicht sicher gelesen werden.",
        )
    })??;
    ensure_session_active(&state, &session).await?;
    let next_cursor = (!page.exhausted).then_some(cursor_id);
    let next_page = (!page.exhausted).then_some(page.sequence.saturating_add(1));
    Ok(Json(DirectoryResponse {
        path: query.path,
        query: filter,
        entries: page.entries,
        next_cursor,
        next_page,
    }))
}

#[derive(Deserialize)]
struct FileQuery {
    path: String,
}

fn parse_range(headers: &HeaderMap, size: u64) -> ApiResult<(u64, u64, bool)> {
    let Some(value) = headers.get(header::RANGE) else {
        return Ok((0, size.saturating_sub(1), false));
    };
    let value = value.to_str().map_err(|_| {
        ApiFailure::new(
            StatusCode::RANGE_NOT_SATISFIABLE,
            "RANGE_INVALID",
            "Ungültiger Downloadbereich.",
        )
    })?;
    let value = value.strip_prefix("bytes=").ok_or_else(|| {
        ApiFailure::new(
            StatusCode::RANGE_NOT_SATISFIABLE,
            "RANGE_INVALID",
            "Nur Byte-Bereiche werden unterstützt.",
        )
    })?;
    if value.contains(',') {
        return Err(ApiFailure::new(
            StatusCode::RANGE_NOT_SATISFIABLE,
            "MULTI_RANGE_UNSUPPORTED",
            "Mehrere Downloadbereiche werden nicht unterstützt.",
        ));
    }
    let (left, right) = value.split_once('-').ok_or_else(|| {
        ApiFailure::new(
            StatusCode::RANGE_NOT_SATISFIABLE,
            "RANGE_INVALID",
            "Ungültiger Downloadbereich.",
        )
    })?;
    let (start, end) = if left.is_empty() {
        let suffix = right.parse::<u64>().map_err(|_| {
            ApiFailure::new(
                StatusCode::RANGE_NOT_SATISFIABLE,
                "RANGE_INVALID",
                "Ungültiger Downloadbereich.",
            )
        })?;
        (size.saturating_sub(suffix), size.saturating_sub(1))
    } else {
        let start = left.parse::<u64>().map_err(|_| {
            ApiFailure::new(
                StatusCode::RANGE_NOT_SATISFIABLE,
                "RANGE_INVALID",
                "Ungültiger Downloadbereich.",
            )
        })?;
        let end = if right.is_empty() {
            size.saturating_sub(1)
        } else {
            right.parse::<u64>().map_err(|_| {
                ApiFailure::new(
                    StatusCode::RANGE_NOT_SATISFIABLE,
                    "RANGE_INVALID",
                    "Ungültiger Downloadbereich.",
                )
            })?
        };
        (start, end.min(size.saturating_sub(1)))
    };
    if size == 0 || start >= size || end < start {
        return Err(ApiFailure::new(
            StatusCode::RANGE_NOT_SATISFIABLE,
            "RANGE_INVALID",
            "Der angeforderte Bereich liegt außerhalb der Datei.",
        ));
    }
    Ok((start, end, true))
}

fn download_headers(
    response: &mut Response,
    file_name: &str,
    size: u64,
    start: u64,
    end: u64,
    partial: bool,
) {
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    let content_length = if size == 0 {
        0
    } else {
        end.saturating_sub(start) + 1
    };
    headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&content_length.to_string()).unwrap(),
    );
    let encoded = utf8_percent_encode(file_name, NON_ALPHANUMERIC).to_string();
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename*=UTF-8''{encoded}"))
            .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
    );
    if partial {
        headers.insert(
            header::CONTENT_RANGE,
            HeaderValue::from_str(&format!("bytes {start}-{end}/{size}")).unwrap(),
        );
    }
}

struct DownloadGuard {
    state: Arc<TransferServiceState>,
    lease: Option<DownloadLease>,
    bytes: u64,
    complete: bool,
}
impl DownloadGuard {
    fn mark_complete(&mut self) {
        self.complete = true;
    }
}
impl Drop for DownloadGuard {
    fn drop(&mut self) {
        if !self.complete {
            let state = self.state.clone();
            let lease = self.lease.take();
            let id = lease.as_ref().map(|lease| lease.id.clone());
            let bytes = self.bytes;
            if let Some(id) = id {
                tokio::spawn(async move {
                    state.finish_download(&id, bytes, "cancelled").await;
                    drop(lease);
                });
            }
        }
    }
}

async fn safe_download_target(
    state: &TransferServiceState,
    address: IpAddr,
    root: PathBuf,
    relative_path: String,
) -> ApiResult<(PathBuf, tokio::fs::File, fs::Metadata)> {
    let anchor = state.download_root_anchor.clone().ok_or_else(|| {
        ApiFailure::new(
            StatusCode::NOT_FOUND,
            "PATH_UNAVAILABLE",
            "Datei ist nicht verfügbar.",
        )
    })?;
    let permit = state
        .begin_filesystem_lookup(address)
        .await
        .ok_or_else(|| {
            ApiFailure::new(
                StatusCode::TOO_MANY_REQUESTS,
                "FILESYSTEM_LIMIT",
                "Es werden bereits zu viele Dateipfade geprüft.",
            )
        })?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let path = safe_existing(&root, &relative_path, Some(false)).map_err(|_| {
            ApiFailure::new(
                StatusCode::NOT_FOUND,
                "PATH_UNAVAILABLE",
                "Datei ist nicht verfügbar.",
            )
        })?;
        let file = fs::File::open(&path).map_err(|_| {
            ApiFailure::new(
                StatusCode::NOT_FOUND,
                "PATH_UNAVAILABLE",
                "Datei ist nicht verfügbar.",
            )
        })?;
        let resolved = anchor.validate_open_file(&file, &path).map_err(|_| {
            ApiFailure::new(
                StatusCode::NOT_FOUND,
                "PATH_UNAVAILABLE",
                "Datei ist nicht verfügbar.",
            )
        })?;
        let metadata = file.metadata().map_err(|_| {
            ApiFailure::new(
                StatusCode::NOT_FOUND,
                "PATH_UNAVAILABLE",
                "Datei ist nicht verfügbar.",
            )
        })?;
        Ok((resolved, tokio::fs::File::from_std(file), metadata))
    })
    .await
    .map_err(|_| {
        ApiFailure::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "FILESYSTEM_TASK_FAILED",
            "Der Dateipfad konnte nicht sicher geprüft werden.",
        )
    })?
}

async fn download(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    query: Result<Query<FileQuery>, QueryRejection>,
) -> ApiResult<Response> {
    let Query(query) = query.map_err(|error| {
        ApiFailure::new(
            error.status(),
            "QUERY_INVALID",
            "Die Dateianfrage ist ungültig.",
        )
    })?;
    let session = authorized(&state, &headers, client.ip(), false).await?;
    let root = state.roots.download.as_ref().cloned().ok_or_else(|| {
        ApiFailure::new(
            StatusCode::NOT_FOUND,
            "DOWNLOAD_DISABLED",
            "Die Downloadfreigabe ist nicht aktiv.",
        )
    })?;
    let (path, mut file, metadata) =
        safe_download_target(&state, client.ip(), root, query.path).await?;
    ensure_session_active(&state, &session).await?;
    let size = metadata.len();
    if size == 0 {
        let mut response = Body::empty().into_response();
        download_headers(
            &mut response,
            path.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("Datei"),
            0,
            0,
            0,
            false,
        );
        return Ok(response);
    }
    let (start, end, partial) = parse_range(&headers, size)?;
    let total = end - start + 1;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Datei")
        .to_string();
    let lease = state
        .begin_download(&session.id, session.address, &name, total)
        .await
        .map_err(|scope| {
            let message = match scope {
                "session" => "Diese Sitzung hat bereits zu viele gleichzeitige Downloads.",
                "address" => "Von dieser Geräteadresse laufen bereits zu viele Downloads.",
                _ => "DMDC verarbeitet bereits die maximale Zahl gleichzeitiger Downloads.",
            };
            ApiFailure::new(StatusCode::TOO_MANY_REQUESTS, "DOWNLOAD_LIMIT", message)
        })?;
    let mut guard = DownloadGuard {
        state: state.clone(),
        lease: Some(lease),
        bytes: 0,
        complete: false,
    };
    if let Err(error) = ensure_session_active(&state, &session).await {
        let id = guard
            .lease
            .as_ref()
            .expect("download lease exists")
            .id
            .clone();
        state.finish_download(&id, 0, "cancelled").await;
        guard.mark_complete();
        return Err(error);
    }
    if file.seek(std::io::SeekFrom::Start(start)).await.is_err() {
        let id = guard
            .lease
            .as_ref()
            .expect("download lease exists")
            .id
            .clone();
        state.finish_download(&id, 0, "failed").await;
        guard.mark_complete();
        return Err(ApiFailure::new(
            StatusCode::BAD_REQUEST,
            "SEEK_FAILED",
            "Downloadposition ist ungültig.",
        ));
    }
    let stream_state = state.clone();
    let stream = async_stream::stream! {
        let mut remaining = total;
        let mut sent = 0_u64;
        let mut cancelled = false;
        let mut expired = false;
        let mut buffer = vec![0_u8; 256 * 1024];
        while remaining > 0 {
            let lease = guard.lease.as_mut().expect("download lease exists");
            if *lease.cancel.borrow() { cancelled = true; break; }
            if lease.expired() { expired = true; break; }
            let requested = buffer.len().min(remaining as usize);
            let lease_remaining = lease.remaining();
            let read_result = tokio::select! {
                _ = lease.cancel.changed() => { cancelled = true; Ok::<usize, std::io::Error>(0) }
                _ = tokio::time::sleep(lease_remaining) => { expired = true; Ok::<usize, std::io::Error>(0) }
                result = file.read(&mut buffer[..requested]) => result,
            };
            let count = match read_result {
                Ok(count) => count,
                Err(error) => {
                    let id = guard.lease.as_ref().expect("download lease exists").id.clone();
                    stream_state.finish_download(&id, sent, "failed").await;
                    guard.mark_complete();
                    yield Err::<Bytes, std::io::Error>(error);
                    return;
                }
            };
            if count == 0 { break; }
            sent += count as u64; remaining -= count as u64; guard.bytes = sent;
            let id = guard.lease.as_ref().expect("download lease exists").id.clone();
            stream_state.update_transfer(&id, sent, None).await;
            yield Ok::<Bytes, std::io::Error>(Bytes::copy_from_slice(&buffer[..count]));
        }
        let final_state = if cancelled { "cancelled" } else if expired { "expired" } else if sent == total { "complete" } else { "failed" };
        let id = guard.lease.as_ref().expect("download lease exists").id.clone();
        stream_state.finish_download(&id, sent, final_state).await;
        guard.mark_complete();
    };
    let mut response = Response::new(Body::from_stream(stream));
    *response.status_mut() = if partial {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::OK
    };
    download_headers(&mut response, &name, size, start, end, partial);
    Ok(response)
}

async fn download_head(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    query: Result<Query<FileQuery>, QueryRejection>,
) -> ApiResult<Response> {
    let Query(query) = query.map_err(|error| {
        ApiFailure::new(
            error.status(),
            "QUERY_INVALID",
            "Die Dateianfrage ist ungültig.",
        )
    })?;
    let session = authorized(&state, &headers, client.ip(), false).await?;
    let root = state.roots.download.as_ref().cloned().ok_or_else(|| {
        ApiFailure::new(
            StatusCode::NOT_FOUND,
            "DOWNLOAD_DISABLED",
            "Die Downloadfreigabe ist nicht aktiv.",
        )
    })?;
    let (path, _file, metadata) =
        safe_download_target(&state, client.ip(), root, query.path).await?;
    ensure_session_active(&state, &session).await?;
    let size = metadata.len();
    let mut response = Body::empty().into_response();
    let (start, end, partial) = if size == 0 {
        (0, 0, false)
    } else {
        parse_range(&headers, size)?
    };
    *response.status_mut() = if partial {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::OK
    };
    download_headers(
        &mut response,
        path.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Datei"),
        size,
        start,
        end,
        partial,
    );
    Ok(response)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateUploadRequest {
    name: String,
    size: u64,
    last_modified: u64,
    #[serde(default)]
    client_token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UploadResponse {
    upload_id: String,
    offset: u64,
    total_bytes: u64,
    chunk_size: usize,
    service_id: String,
    last_modified: u64,
}

async fn create_upload(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    payload: Result<Json<CreateUploadRequest>, JsonRejection>,
) -> ApiResult<Json<UploadResponse>> {
    let Json(payload) = payload.map_err(|error| {
        ApiFailure::new(
            error.status(),
            "UPLOAD_BODY_INVALID",
            "Die Uploadinformationen sind ungültig.",
        )
    })?;
    let session = authorized(&state, &headers, client.ip(), true).await?;
    let root = state.roots.upload.as_ref().ok_or_else(|| {
        ApiFailure::new(
            StatusCode::NOT_FOUND,
            "UPLOAD_DISABLED",
            "Der Upload-Eingang ist nicht aktiv.",
        )
    })?;
    if state
        .settings
        .max_upload_bytes
        .is_some_and(|limit| payload.size > limit)
    {
        return Err(ApiFailure::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "FILE_TOO_LARGE",
            "Die Datei überschreitet das konfigurierte Uploadlimit.",
        ));
    }
    let name = safe_file_name_for_root(root, &payload.name)
        .map_err(|message| ApiFailure::new(StatusCode::BAD_REQUEST, "NAME_INVALID", message))?;
    let client_token = if payload.client_token.is_empty() {
        Uuid::new_v4().to_string()
    } else if payload.client_token.len() == 32
        && payload
            .client_token
            .bytes()
            .all(|value| value.is_ascii_hexdigit())
    {
        payload.client_token.clone()
    } else {
        return Err(ApiFailure::new(
            StatusCode::BAD_REQUEST,
            "UPLOAD_TOKEN_INVALID",
            "Die Wiederaufnahme-ID des Uploads ist ungültig.",
        ));
    };
    if let Some(completed) = state
        .completed_upload_by_token(&client_token, &name, payload.size, payload.last_modified)
        .await
    {
        return Ok(Json(UploadResponse {
            upload_id: completed.upload_id,
            offset: completed.total_bytes,
            total_bytes: completed.total_bytes,
            chunk_size: CHUNK_SIZE,
            service_id: state.service_id.clone(),
            last_modified: completed.last_modified,
        }));
    }
    let _reservation = state.upload_fs_lock.lock().await;
    ensure_session_active(&state, &session).await?;
    if !state.roots_are_current() {
        return Err(ApiFailure::new(
            StatusCode::CONFLICT,
            "UPLOAD_ROOT_CHANGED",
            "Der Upload-Eingang wurde während des Betriebs verändert.",
        ));
    }
    state.refresh_inbox_usage().map_err(|_| {
        ApiFailure::new(
            StatusCode::INSUFFICIENT_STORAGE,
            "INBOX_USAGE_UNKNOWN",
            "Die Belegung des Upload-Eingangs konnte nicht sicher bestimmt werden.",
        )
    })?;
    let active_uploads: Vec<_> = state.uploads.lock().await.values().cloned().collect();
    if active_uploads.len() >= 64 {
        return Err(ApiFailure::new(
            StatusCode::TOO_MANY_REQUESTS,
            "TOO_MANY_UPLOADS",
            "Es sind bereits zu viele unvollständige Uploads vorhanden.",
        ));
    }
    let mut address_uploads = 0_usize;
    for upload in &active_uploads {
        let upload = upload.lock().await;
        if !upload.cancelled
            && upload.owner_session == session.id
            && upload.client_token == client_token
            && upload.name == name
            && upload.declared_size == payload.size
            && upload.last_modified == payload.last_modified
        {
            return Ok(Json(UploadResponse {
                upload_id: upload.id.clone(),
                offset: upload.offset,
                total_bytes: upload.declared_size,
                chunk_size: CHUNK_SIZE,
                service_id: state.service_id.clone(),
                last_modified: upload.last_modified,
            }));
        }
        if !upload.cancelled && upload.owner_address == session.address {
            address_uploads += 1;
        }
    }
    if address_uploads >= MAX_UPLOADS_PER_ADDRESS {
        return Err(ApiFailure::new(
            StatusCode::TOO_MANY_REQUESTS,
            "UPLOAD_CLIENT_LIMIT",
            "Von dieser Geräteadresse sind bereits zu viele unvollständige Uploads vorhanden.",
        ));
    }
    let space_permit = state
        .begin_filesystem_lookup(client.ip())
        .await
        .ok_or_else(|| {
            ApiFailure::new(
                StatusCode::TOO_MANY_REQUESTS,
                "FILESYSTEM_LIMIT",
                "Es werden bereits zu viele Dateisysteminformationen geprüft.",
            )
        })?;
    let space_root = root.clone();
    let available = tokio::task::spawn_blocking(move || {
        let _permit = space_permit;
        fs2::available_space(space_root)
    })
    .await
    .map_err(|_| {
        ApiFailure::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "FILESYSTEM_TASK_FAILED",
            "Die Speicherprüfung konnte nicht sicher ausgeführt werden.",
        )
    })?
    .map_err(|_| {
        ApiFailure::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "SPACE_UNKNOWN",
            "Freier Speicher konnte nicht geprüft werden.",
        )
    })?;
    if available < DISK_RESERVE {
        return Err(ApiFailure::new(
            StatusCode::INSUFFICIENT_STORAGE,
            "DISK_FULL",
            "Die Sicherheitsreserve des Datenträgers ist erreicht.",
        ));
    }
    ensure_session_active(&state, &session).await?;
    let id = Uuid::new_v4().to_string();
    let partial_dir = state.partial_dir.as_ref().ok_or_else(|| {
        ApiFailure::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "PARTIAL_DIR_FAILED",
            "Temporärer Uploadordner ist nicht verfügbar.",
        )
    })?;
    let partial_path = partial_dir.join(format!("{id}.part"));
    let transfer_id = Uuid::new_v4().to_string();
    let mut uploads = state.uploads.lock().await;
    state.reserve_upload_object().map_err(|_| {
        ApiFailure::new(
            StatusCode::INSUFFICIENT_STORAGE,
            "INBOX_FILE_LIMIT",
            "Der Upload-Eingang hat die konfigurierte Dateianzahl erreicht.",
        )
    })?;
    let partial_file = match create_upload_partial(&partial_path) {
        Ok(file) => file,
        Err(_) => {
            state.release_upload(0);
            return Err(ApiFailure::new(
                StatusCode::CONFLICT,
                "UPLOAD_COLLISION",
                "Upload konnte nicht angelegt werden.",
            ));
        }
    };
    let transfer_name = name.clone();
    let record = UploadRecord {
        id: id.clone(),
        owner_session: session.id,
        owner_address: session.address,
        name,
        declared_size: payload.size,
        offset: 0,
        last_modified: payload.last_modified,
        client_token,
        created_at: Instant::now(),
        last_activity: Instant::now(),
        cancelled: false,
        finalizing: false,
        cancel_signal: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        chunk_slots: Arc::new(tokio::sync::Semaphore::new(1)),
        partial_path,
        partial_file,
        transfer_id: transfer_id.clone(),
    };
    uploads.insert(id.clone(), Arc::new(Mutex::new(record)));
    drop(uploads);
    state
        .record_transfer_with_id(&transfer_id, "upload", &transfer_name, payload.size)
        .await;
    Ok(Json(UploadResponse {
        upload_id: id,
        offset: 0,
        total_bytes: payload.size,
        chunk_size: CHUNK_SIZE,
        service_id: state.service_id.clone(),
        last_modified: payload.last_modified,
    }))
}

async fn owned_upload(
    state: &TransferServiceState,
    id: &str,
    session: &SessionRecord,
) -> ApiResult<Arc<Mutex<UploadRecord>>> {
    let record = state.uploads.lock().await.get(id).cloned().ok_or_else(|| {
        ApiFailure::new(
            StatusCode::NOT_FOUND,
            "UPLOAD_NOT_FOUND",
            "Teilübertragung wurde nicht gefunden.",
        )
    })?;
    let record_guard = record.lock().await;
    if record_guard.cancelled {
        return Err(ApiFailure::new(
            StatusCode::NOT_FOUND,
            "UPLOAD_NOT_FOUND",
            "Teilübertragung wurde nicht gefunden.",
        ));
    }
    if record_guard.owner_session != session.id {
        return Err(ApiFailure::new(
            StatusCode::FORBIDDEN,
            "UPLOAD_OWNER_MISMATCH",
            "Diese Teilübertragung gehört zu einer anderen Sitzung.",
        ));
    }
    drop(record_guard);
    Ok(record)
}

async fn upload_status(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    id: Result<AxumPath<String>, PathRejection>,
) -> ApiResult<Json<UploadResponse>> {
    let AxumPath(id) = id.map_err(|error| {
        ApiFailure::new(
            error.status(),
            "UPLOAD_ID_INVALID",
            "Die Upload-ID ist ungültig.",
        )
    })?;
    let session = authorized(&state, &headers, client.ip(), false).await?;
    if let Some(completed) = state.completed_upload(&id, session.address).await {
        return Ok(Json(UploadResponse {
            upload_id: completed.upload_id,
            offset: completed.total_bytes,
            total_bytes: completed.total_bytes,
            chunk_size: CHUNK_SIZE,
            service_id: state.service_id.clone(),
            last_modified: completed.last_modified,
        }));
    }
    let record = owned_upload(&state, &id, &session).await?;
    let record = record.lock().await;
    Ok(Json(UploadResponse {
        upload_id: record.id.clone(),
        offset: record.offset,
        total_bytes: record.declared_size,
        chunk_size: CHUNK_SIZE,
        service_id: state.service_id.clone(),
        last_modified: record.last_modified,
    }))
}

async fn upload_chunk(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    id: Result<AxumPath<String>, PathRejection>,
    Extension(_chunk_permit): Extension<UploadChunkPermit>,
    body: Result<Bytes, BytesRejection>,
) -> ApiResult<Response> {
    let AxumPath(id) = id.map_err(|error| {
        ApiFailure::new(
            error.status(),
            "UPLOAD_ID_INVALID",
            "Die Upload-ID ist ungültig.",
        )
    })?;
    let session = authorized(&state, &headers, client.ip(), true).await?;
    let body = body.map_err(|error| {
        ApiFailure::new(
            error.status(),
            "CHUNK_BODY_INVALID",
            "Der Uploadblock konnte nicht gelesen werden.",
        )
    })?;
    if body.is_empty() {
        return Err(ApiFailure::new(
            StatusCode::BAD_REQUEST,
            "EMPTY_CHUNK",
            "Ein Uploadblock darf nicht leer sein.",
        ));
    }
    if body.len() > CHUNK_SIZE {
        return Err(ApiFailure::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "CHUNK_TOO_LARGE",
            "Der Uploadblock ist größer als erlaubt.",
        ));
    }
    let supplied_offset = headers
        .get("upload-offset")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| {
            ApiFailure::new(
                StatusCode::BAD_REQUEST,
                "OFFSET_MISSING",
                "Upload-Offset fehlt.",
            )
        })?;
    let record = owned_upload(&state, &id, &session).await?;
    let mut record = record.lock().await;
    if record.cancelled {
        return Err(ApiFailure::new(
            StatusCode::NOT_FOUND,
            "UPLOAD_NOT_FOUND",
            "Teilübertragung wurde nicht gefunden.",
        ));
    }
    if record.finalizing {
        return Err(ApiFailure::new(
            StatusCode::CONFLICT,
            "UPLOAD_FINALIZING",
            "Die Datei wird bereits endgültig übernommen.",
        ));
    }
    ensure_session_active(&state, &session).await?;
    if supplied_offset != record.offset {
        return Err(ApiFailure::new(
            StatusCode::CONFLICT,
            "OFFSET_MISMATCH",
            format!("Erwarteter Offset: {}", record.offset),
        ));
    }
    if record.offset.saturating_add(body.len() as u64) > record.declared_size {
        return Err(ApiFailure::new(
            StatusCode::BAD_REQUEST,
            "SIZE_EXCEEDED",
            "Der Upload überschreitet die angekündigte Dateigröße.",
        ));
    }
    let remaining = record.declared_size.saturating_sub(record.offset);
    let required = remaining.min(CHUNK_SIZE as u64) as usize;
    if body.len() != required {
        return Err(ApiFailure::new(
            StatusCode::BAD_REQUEST,
            "CHUNK_SIZE_INVALID",
            format!("Für diesen Uploadblock werden genau {required} Bytes erwartet."),
        ));
    }
    state.reserve_upload_bytes(body.len() as u64).map_err(|_| {
        ApiFailure::new(
            StatusCode::INSUFFICIENT_STORAGE,
            "INBOX_BYTE_LIMIT",
            "Der Upload-Eingang hat das konfigurierte Speicherlimit erreicht.",
        )
    })?;
    let write_result = (|| -> std::io::Result<()> {
        if !state.roots_are_current() {
            return Err(std::io::Error::other("upload root changed"));
        }
        let anchor = state
            .partial_dir_anchor
            .as_ref()
            .ok_or_else(|| std::io::Error::other("partial root unavailable"))?;
        anchor
            .validate_open_file(&record.partial_file, &record.partial_path)
            .map_err(std::io::Error::other)?;
        let upload_root = state
            .roots
            .upload
            .as_ref()
            .ok_or_else(|| std::io::Error::other("upload root unavailable"))?;
        if fs2::available_space(upload_root)? < DISK_RESERVE.saturating_add(body.len() as u64) {
            return Err(std::io::Error::other("disk reserve reached"));
        }
        let offset = record.offset;
        record.partial_file.seek(std::io::SeekFrom::Start(offset))?;
        record.partial_file.write_all(&body)?;
        record.partial_file.sync_data()
    })();
    if write_result.is_err() {
        state.release_upload_bytes(body.len() as u64);
        return Err(ApiFailure::new(
            StatusCode::INSUFFICIENT_STORAGE,
            "PARTIAL_WRITE_FAILED",
            "Uploadblock konnte nicht sicher gespeichert werden.",
        ));
    }
    record.offset += body.len() as u64;
    record.last_activity = Instant::now();
    state
        .update_transfer(&record.transfer_id, record.offset, None)
        .await;
    let mut response = StatusCode::OK.into_response();
    response.headers_mut().insert(
        "upload-offset",
        HeaderValue::from_str(&record.offset.to_string()).unwrap(),
    );
    Ok(response)
}

#[derive(Clone, Serialize)]
struct CompleteResponse {
    name: String,
}

async fn finalize_upload_owned(
    state: Arc<TransferServiceState>,
    id: String,
    record: Arc<Mutex<UploadRecord>>,
) -> ApiResult<CompleteResponse> {
    let result = async {
        let (
            partial_path,
            requested_name,
            transfer_id,
            total,
            owner_address,
            last_modified,
            client_token,
            cancel_signal,
            file,
            root,
            anchor,
        ) = {
            let record = record.lock().await;
            if record.cancelled {
                return Err(ApiFailure::new(
                    StatusCode::NOT_FOUND,
                    "UPLOAD_NOT_FOUND",
                    "Teilübertragung wurde nicht gefunden.",
                ));
            }
            if record.offset != record.declared_size {
                return Err(ApiFailure::new(
                    StatusCode::CONFLICT,
                    "UPLOAD_INCOMPLETE",
                    "Die Datei ist noch nicht vollständig übertragen.",
                ));
            }
            let actual_size = record
                .partial_file
                .metadata()
                .map_err(|_| {
                    ApiFailure::new(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "PARTIAL_STAT_FAILED",
                        "Die Größe der Teilübertragung konnte nicht geprüft werden.",
                    )
                })?
                .len();
            if actual_size != record.declared_size {
                return Err(ApiFailure::new(
                    StatusCode::CONFLICT,
                    "FINAL_SIZE_MISMATCH",
                    "Die gespeicherte Dateigröße stimmt nicht mit dem Upload überein.",
                ));
            }
            let root = state.roots.upload.as_ref().cloned().ok_or_else(|| {
                ApiFailure::new(
                    StatusCode::NOT_FOUND,
                    "UPLOAD_DISABLED",
                    "Der Upload-Eingang ist nicht aktiv.",
                )
            })?;
            let anchor = state.upload_root_anchor.clone().ok_or_else(|| {
                ApiFailure::new(
                    StatusCode::CONFLICT,
                    "UPLOAD_ROOT_CHANGED",
                    "Der Upload-Eingang wurde während des Betriebs verändert.",
                )
            })?;
            (
                record.partial_path.clone(),
                record.name.clone(),
                record.transfer_id.clone(),
                record.declared_size,
                record.owner_address,
                record.last_modified,
                record.client_token.clone(),
                record.cancel_signal.clone(),
                record.partial_file.try_clone().map_err(|_| {
                    ApiFailure::new(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "PARTIAL_OPEN_FAILED",
                        "Teilübertragung konnte nicht sicher übernommen werden.",
                    )
                })?,
                root,
                anchor,
            )
        };
        let publish_name = requested_name.clone();
        let publish_path = partial_path.clone();
        let publish_cancel = cancel_signal.clone();
        let target = tokio::task::spawn_blocking(move || -> std::io::Result<PathBuf> {
            use std::sync::atomic::Ordering;
            if publish_cancel.load(Ordering::Acquire) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "upload cancelled",
                ));
            }
            file.sync_all()?;
            if publish_cancel.load(Ordering::Acquire) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "upload cancelled",
                ));
            }
            let target = publish_open_upload(&file, &publish_path, &root, &anchor, &publish_name)
                .map_err(std::io::Error::other)?;
            if publish_cancel.load(Ordering::Acquire) {
                let _ = delete_open_upload(&file, &target);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "upload cancelled",
                ));
            }
            Ok(target)
        })
        .await
        .map_err(|_| {
            ApiFailure::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FINALIZE_FAILED",
                "Datei konnte nicht endgültig übernommen werden.",
            )
        })?
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::Interrupted {
                ApiFailure::new(
                    StatusCode::NOT_FOUND,
                    "UPLOAD_NOT_FOUND",
                    "Teilübertragung wurde abgebrochen.",
                )
            } else {
                ApiFailure::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "FINALIZE_FAILED",
                    "Datei konnte nicht endgültig übernommen werden.",
                )
            }
        })?;
        let final_name = target
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Datei")
            .to_string();
        let _filesystem = state.upload_fs_lock.lock().await;
        if cancel_signal.load(std::sync::atomic::Ordering::Acquire) {
            return Err(ApiFailure::new(
                StatusCode::NOT_FOUND,
                "UPLOAD_NOT_FOUND",
                "Teilübertragung wurde abgebrochen.",
            ));
        }
        let still_active = state
            .uploads
            .lock()
            .await
            .get(&id)
            .is_some_and(|current| Arc::ptr_eq(current, &record));
        if !still_active {
            return Err(ApiFailure::new(
                StatusCode::NOT_FOUND,
                "UPLOAD_NOT_FOUND",
                "Teilübertragung wurde abgebrochen.",
            ));
        }
        state.uploads.lock().await.remove(&id);
        state.complete_upload_budget(total);
        drop(_filesystem);
        state
            .remember_completed_upload(CompletedUpload::new(
                id.clone(),
                owner_address,
                final_name.clone(),
                requested_name,
                total,
                last_modified,
                client_token,
            ))
            .await;
        state
            .update_transfer(&transfer_id, total, Some("complete"))
            .await;
        Ok(CompleteResponse { name: final_name })
    }
    .await;

    if result.is_err() {
        record.lock().await.finalizing = false;
    }
    result
}

async fn complete_upload(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    id: Result<AxumPath<String>, PathRejection>,
) -> ApiResult<Json<CompleteResponse>> {
    let AxumPath(id) = id.map_err(|error| {
        ApiFailure::new(
            error.status(),
            "UPLOAD_ID_INVALID",
            "Die Upload-ID ist ungültig.",
        )
    })?;
    let session = authorized(&state, &headers, client.ip(), true).await?;
    if let Some(completed) = state.completed_upload(&id, session.address).await {
        return Ok(Json(CompleteResponse {
            name: completed.name,
        }));
    }
    let filesystem = state.upload_fs_lock.lock().await;
    ensure_session_active(&state, &session).await?;
    let record = owned_upload(&state, &id, &session).await?;
    let mut record_guard = record.lock().await;
    if record_guard.cancelled {
        return Err(ApiFailure::new(
            StatusCode::NOT_FOUND,
            "UPLOAD_NOT_FOUND",
            "Teilübertragung wurde nicht gefunden.",
        ));
    }
    if record_guard.offset != record_guard.declared_size {
        return Err(ApiFailure::new(
            StatusCode::CONFLICT,
            "UPLOAD_INCOMPLETE",
            "Die Datei ist noch nicht vollständig übertragen.",
        ));
    }
    if record_guard.finalizing {
        return Err(ApiFailure::new(
            StatusCode::CONFLICT,
            "UPLOAD_FINALIZING",
            "Die Datei wird bereits endgültig übernommen.",
        ));
    }
    record_guard.finalizing = true;
    drop(record_guard);
    drop(filesystem);
    let task_state = state.clone();
    let task_id = id.clone();
    let task_record = record.clone();
    let commit =
        tokio::spawn(async move { finalize_upload_owned(task_state, task_id, task_record).await });
    match commit.await {
        Ok(result) => result.map(Json),
        Err(_) => {
            record.lock().await.finalizing = false;
            Err(ApiFailure::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FINALIZE_FAILED",
                "Datei konnte nicht endgültig übernommen werden.",
            ))
        }
    }
}

async fn cancel_upload(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    id: Result<AxumPath<String>, PathRejection>,
) -> ApiResult<StatusCode> {
    let AxumPath(id) = id.map_err(|error| {
        ApiFailure::new(
            error.status(),
            "UPLOAD_ID_INVALID",
            "Die Upload-ID ist ungültig.",
        )
    })?;
    let session = authorized(&state, &headers, client.ip(), true).await?;
    let _cancel = state.upload_fs_lock.lock().await;
    let record = owned_upload(&state, &id, &session).await?;
    let mut record = record.lock().await;
    record.cancelled = true;
    record
        .cancel_signal
        .store(true, std::sync::atomic::Ordering::Release);
    let path = record.partial_path.clone();
    let transfer_id = record.transfer_id.clone();
    let offset = record.offset;
    let _ = delete_open_upload(&record.partial_file, &path);
    drop(record);
    state.uploads.lock().await.remove(&id);
    state.release_upload(offset);
    state
        .update_transfer(&transfer_id, offset, Some("cancelled"))
        .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn static_asset(uri: axum::http::Uri) -> Response {
    let requested = uri.path().trim_start_matches('/');
    if requested.starts_with("api/") {
        return ApiFailure::new(StatusCode::NOT_FOUND, "NOT_FOUND", "Nicht gefunden.")
            .into_response();
    }
    let asset_name = if requested.is_empty() {
        "index.html"
    } else {
        requested
    };
    let asset = MobileAssets::get(asset_name);
    match asset {
        Some(asset) => {
            let mime = mime_guess::from_path(asset_name).first_or_octet_stream();
            let mut response = Body::from(asset.data.into_owned()).into_response();
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime.as_ref())
                    .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
            );
            response
        }
        None => ApiFailure::new(
            StatusCode::NOT_FOUND,
            "MOBILE_ASSETS_MISSING",
            "Mobile Oberfläche wurde nicht eingebettet.",
        )
        .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        network::NetworkInterfaceInfo,
        settings::{AppSettings, ShareSettings},
        shares::ShareRoots,
    };
    use axum::body::to_bytes;
    use std::net::Ipv4Addr;
    use tower::ServiceExt;

    fn test_state(root: &Path) -> Arc<TransferServiceState> {
        // Production roots are canonicalized by `prepare_roots`. Mirror that
        // invariant here as Windows may otherwise compare a `\\?\` path with
        // the non-canonical temporary-directory spelling.
        let root = fs::canonicalize(root).unwrap();
        let settings = AppSettings {
            download_share: ShareSettings {
                enabled: true,
                path: root.display().to_string(),
            },
            upload_share: ShareSettings {
                enabled: true,
                path: root.display().to_string(),
            },
            ..AppSettings::default()
        };
        let interface = NetworkInterfaceInfo {
            id: "lan|192.168.10.2".into(),
            name: "lan".into(),
            profile_name: "Testnetz".into(),
            address: Ipv4Addr::new(192, 168, 10, 2),
            prefix_length: 24,
            network_id: "lan-test".into(),
            category: "Privat".into(),
            profile_resolved: true,
            preferred: true,
            netmask: Ipv4Addr::new(255, 255, 255, 0),
        };
        let state = Arc::new(
            TransferServiceState::new(
                settings,
                interface,
                ShareRoots {
                    download: Some(root.clone()),
                    upload: Some(root),
                },
                None,
            )
            .unwrap(),
        );
        *state.access_code.write().unwrap() = "12345678".into();
        state
    }

    fn request(method: Method, uri: &str, body: Body) -> Request<Body> {
        request_from(method, uri, body, Ipv4Addr::new(192, 168, 10, 50))
    }

    fn request_from(method: Method, uri: &str, body: Body, address: Ipv4Addr) -> Request<Body> {
        let mut builder = Request::builder()
            .method(method.clone())
            .uri(uri)
            .header(header::HOST, "192.168.10.2:8765");
        if matches!(
            method,
            Method::POST | Method::PATCH | Method::DELETE | Method::PUT
        ) {
            builder = builder.header(header::ORIGIN, "http://192.168.10.2:8765");
        }
        let mut request = builder.body(body).unwrap();
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from((address, 54000))));
        request
    }

    async fn json(response: Response) -> serde_json::Value {
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    async fn session_headers_for(
        state: &TransferServiceState,
        address: Ipv4Addr,
    ) -> (String, String) {
        let session = state
            .create_session(IpAddr::V4(address), "Testbrowser".into())
            .await
            .unwrap();
        (format!("dmdc_session={}", session.token), session.csrf)
    }

    async fn session_headers(state: &TransferServiceState) -> (String, String) {
        session_headers_for(state, Ipv4Addr::new(192, 168, 10, 50)).await
    }

    #[test]
    fn parses_download_ranges() {
        let mut headers = HeaderMap::new();
        headers.insert(header::RANGE, HeaderValue::from_static("bytes=10-19"));
        assert_eq!(parse_range(&headers, 100).unwrap(), (10, 19, true));
        headers.insert(header::RANGE, HeaderValue::from_static("bytes=-7"));
        assert_eq!(parse_range(&headers, 100).unwrap(), (93, 99, true));
        headers.insert(header::RANGE, HeaderValue::from_static("bytes=100-"));
        assert!(parse_range(&headers, 100).is_err());
    }

    #[test]
    fn sorts_names_naturally_without_case() {
        assert_eq!(
            natural_compare("Datei 2.txt", "datei 10.txt"),
            Ordering::Less
        );
        assert_eq!(natural_compare("BILD.JPG", "bild.jpg"), Ordering::Equal);
    }

    #[tokio::test]
    async fn authenticates_and_sets_http_only_cookie() {
        let temp = tempfile::tempdir().unwrap();
        let app = router(test_state(temp.path()));
        let mut auth = request(
            Method::POST,
            "/api/v1/auth",
            Body::from(r#"{"code":"12345678"}"#),
        );
        auth.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        let response = app.oneshot(auth).await.unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(!cookie.contains("Secure"));
    }

    #[tokio::test]
    async fn rejects_wrong_host_and_outside_subnet() {
        let temp = tempfile::tempdir().unwrap();
        let app = router(test_state(temp.path()));
        let mut wrong_host = request(Method::GET, "/", Body::empty());
        wrong_host
            .headers_mut()
            .insert(header::HOST, HeaderValue::from_static("example.test"));
        assert_eq!(
            app.clone().oneshot(wrong_host).await.unwrap().status(),
            StatusCode::MISDIRECTED_REQUEST
        );

        let mut outside = request(Method::GET, "/", Body::empty());
        outside
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from((
                Ipv4Addr::new(192, 168, 11, 5),
                54000,
            ))));
        assert_eq!(
            app.oneshot(outside).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn enforces_csrf_for_writes() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path());
        let (cookie, _) = session_headers(&state).await;
        let app = router(state);
        let mut logout = request(Method::POST, "/api/v1/logout", Body::empty());
        logout
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        let response = app.oneshot(logout).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(json(response).await["code"], "CSRF_INVALID");
    }

    #[tokio::test]
    async fn locks_ip_on_the_tenth_wrong_code() {
        let temp = tempfile::tempdir().unwrap();
        let app = router(test_state(temp.path()));
        for _ in 0..9 {
            let mut auth = request(
                Method::POST,
                "/api/v1/auth",
                Body::from(r#"{"code":"00000000"}"#),
            );
            auth.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            assert_eq!(
                app.clone().oneshot(auth).await.unwrap().status(),
                StatusCode::UNAUTHORIZED
            );
        }
        let mut tenth = request(
            Method::POST,
            "/api/v1/auth",
            Body::from(r#"{"code":"00000000"}"#),
        );
        tenth.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        assert_eq!(
            app.clone().oneshot(tenth).await.unwrap().status(),
            StatusCode::TOO_MANY_REQUESTS
        );
        let mut blocked = request(
            Method::POST,
            "/api/v1/auth",
            Body::from(r#"{"code":"12345678"}"#),
        );
        blocked.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        assert_eq!(
            app.oneshot(blocked).await.unwrap().status(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    #[tokio::test]
    async fn serializes_concurrent_failed_code_attempts() {
        let temp = tempfile::tempdir().unwrap();
        let app = router(test_state(temp.path()));
        let mut tasks = tokio::task::JoinSet::new();
        for _ in 0..20 {
            let app = app.clone();
            tasks.spawn(async move {
                let mut auth = request(
                    Method::POST,
                    "/api/v1/auth",
                    Body::from(r#"{"code":"00000000"}"#),
                );
                auth.headers_mut().insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
                app.oneshot(auth).await.unwrap().status()
            });
        }
        let mut unauthorized = 0;
        let mut blocked = 0;
        while let Some(result) = tasks.join_next().await {
            match result.unwrap() {
                StatusCode::UNAUTHORIZED => unauthorized += 1,
                StatusCode::TOO_MANY_REQUESTS => blocked += 1,
                status => panic!("unerwarteter Status: {status}"),
            }
        }
        assert_eq!(unauthorized, 9);
        assert_eq!(blocked, 11);
    }

    #[tokio::test]
    async fn distributed_failures_block_without_rotating_the_service_code() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path());
        let app = router(state.clone());
        for index in 1..50 {
            let mut auth = request_from(
                Method::POST,
                "/api/v1/auth",
                Body::from(r#"{"code":"00000000"}"#),
                Ipv4Addr::new(192, 168, 10, index),
            );
            auth.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            assert_eq!(
                app.clone().oneshot(auth).await.unwrap().status(),
                StatusCode::UNAUTHORIZED
            );
        }
        let mut threshold = request_from(
            Method::POST,
            "/api/v1/auth",
            Body::from(r#"{"code":"00000000"}"#),
            Ipv4Addr::new(192, 168, 10, 50),
        );
        threshold.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        let response = app.clone().oneshot(threshold).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(json(response).await["code"], "SERVICE_CODE_BLOCKED");
        let unchanged = state.access_code.read().unwrap().clone();
        assert_eq!(unchanged, "12345678");

        let mut recovery = request_from(
            Method::POST,
            "/api/v1/auth",
            Body::from(format!(r#"{{"code":"{unchanged}"}}"#)),
            Ipv4Addr::new(192, 168, 10, 51),
        );
        recovery.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        assert_eq!(
            app.oneshot(recovery).await.unwrap().status(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    #[tokio::test]
    async fn rejects_patch_before_polling_an_unauthenticated_body() {
        let temp = tempfile::tempdir().unwrap();
        let body = Body::from_stream(async_stream::stream! {
            if std::hint::black_box(true) {
                panic!("unauthenticated PATCH body must not be polled");
            }
            yield Ok::<Bytes, std::io::Error>(Bytes::from_static(b"blocked"));
        });
        let patch = request(Method::PATCH, "/api/v1/uploads/not-owned", body);
        let response = router(test_state(temp.path()))
            .oneshot(patch)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(json(response).await["code"], "AUTH_REQUIRED");
    }

    #[tokio::test]
    async fn rejects_unknown_owned_patch_before_polling_its_authenticated_body() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path());
        let (cookie, csrf) = session_headers(&state).await;
        let body = Body::from_stream(async_stream::stream! {
            if std::hint::black_box(true) {
                panic!("unknown authenticated PATCH body must not be polled");
            }
            yield Ok::<Bytes, std::io::Error>(Bytes::from_static(b"blocked"));
        });
        let mut patch = request(Method::PATCH, "/api/v1/uploads/not-owned", body);
        patch
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        patch
            .headers_mut()
            .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
        let response = router(state).oneshot(patch).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(json(response).await["code"], "UPLOAD_NOT_FOUND");
    }

    #[tokio::test]
    async fn rejects_foreign_patch_before_polling_its_authenticated_body() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path());
        let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
        let owner = state.create_session(address, "owner".into()).await.unwrap();
        let attacker = state
            .create_session(address, "attacker".into())
            .await
            .unwrap();
        let id = Uuid::new_v4().to_string();
        let partial_path = state
            .partial_dir
            .as_ref()
            .unwrap()
            .join(format!("{id}.part"));
        let partial_file = create_upload_partial(&partial_path).unwrap();
        state.uploads.lock().await.insert(
            id.clone(),
            Arc::new(Mutex::new(UploadRecord {
                id: id.clone(),
                owner_session: owner.id,
                owner_address: address,
                name: "foreign.bin".into(),
                declared_size: 1,
                offset: 0,
                last_modified: 1,
                client_token: "foreign-token".into(),
                created_at: Instant::now(),
                last_activity: Instant::now(),
                cancelled: false,
                finalizing: false,
                cancel_signal: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                chunk_slots: Arc::new(tokio::sync::Semaphore::new(1)),
                partial_path,
                partial_file,
                transfer_id: "transfer".into(),
            })),
        );
        let body = Body::from_stream(async_stream::stream! {
            if std::hint::black_box(true) {
                panic!("foreign authenticated PATCH body must not be polled");
            }
            yield Ok::<Bytes, std::io::Error>(Bytes::from_static(b"blocked"));
        });
        let mut patch = request(Method::PATCH, &format!("/api/v1/uploads/{id}"), body);
        patch.headers_mut().insert(
            header::COOKIE,
            HeaderValue::from_str(&format!("dmdc_session={}", attacker.token)).unwrap(),
        );
        patch.headers_mut().insert(
            "x-dmdc-csrf",
            HeaderValue::from_str(&attacker.csrf).unwrap(),
        );
        let response = router(state).oneshot(patch).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(json(response).await["code"], "UPLOAD_OWNER_MISMATCH");
    }

    #[tokio::test]
    async fn rejects_parallel_patch_before_polling_a_second_body_for_the_same_upload() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path());
        let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
        let session = state.create_session(address, "owner".into()).await.unwrap();
        let id = Uuid::new_v4().to_string();
        let partial_path = state
            .partial_dir
            .as_ref()
            .unwrap()
            .join(format!("{id}.part"));
        let partial_file = create_upload_partial(&partial_path).unwrap();
        let record = Arc::new(Mutex::new(UploadRecord {
            id: id.clone(),
            owner_session: session.id,
            owner_address: address,
            name: "parallel.bin".into(),
            declared_size: 1,
            offset: 0,
            last_modified: 1,
            client_token: "parallel-token".into(),
            created_at: Instant::now(),
            last_activity: Instant::now(),
            cancelled: false,
            finalizing: false,
            cancel_signal: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            chunk_slots: Arc::new(tokio::sync::Semaphore::new(1)),
            partial_path,
            partial_file,
            transfer_id: "transfer".into(),
        }));
        state
            .uploads
            .lock()
            .await
            .insert(id.clone(), record.clone());
        let active_chunk = {
            let record = record.lock().await;
            state.begin_upload_chunk(&record).unwrap()
        };
        let body = Body::from_stream(async_stream::stream! {
            if std::hint::black_box(true) {
                panic!("parallel PATCH body must not be polled");
            }
            yield Ok::<Bytes, std::io::Error>(Bytes::from_static(b"blocked"));
        });
        let mut patch = request(Method::PATCH, &format!("/api/v1/uploads/{id}"), body);
        patch.headers_mut().insert(
            header::COOKIE,
            HeaderValue::from_str(&format!("dmdc_session={}", session.token)).unwrap(),
        );
        patch
            .headers_mut()
            .insert("x-dmdc-csrf", HeaderValue::from_str(&session.csrf).unwrap());
        patch
            .headers_mut()
            .insert("upload-offset", HeaderValue::from_static("0"));

        let response = router(state).oneshot(patch).await.unwrap();
        drop(active_chunk);
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(json(response).await["code"], "UPLOAD_CHUNK_BUSY");
    }

    #[tokio::test]
    async fn directory_page_stops_after_a_bounded_number_of_hidden_entries() {
        let temp = tempfile::tempdir().unwrap();
        for index in 0..(DIRECTORY_SCAN_LIMIT + 40) {
            fs::write(temp.path().join(format!(".hidden-{index:04}")), b"x").unwrap();
        }
        let state = test_state(temp.path());
        let (cookie, _) = session_headers(&state).await;
        let app = router(state);
        let mut list = request(Method::GET, "/api/v1/downloads?path=", Body::empty());
        list.headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        let response = json(app.oneshot(list).await.unwrap()).await;
        assert!(response["entries"].as_array().unwrap().is_empty());
        assert!(response["nextCursor"].as_str().is_some());
    }

    #[tokio::test]
    async fn directory_cursor_is_bound_to_its_session() {
        let temp = tempfile::tempdir().unwrap();
        for index in 0..(DIRECTORY_SCAN_LIMIT + 1) {
            fs::write(temp.path().join(format!(".hidden-{index:04}")), b"x").unwrap();
        }
        let state = test_state(temp.path());
        let (first_cookie, _) = session_headers(&state).await;
        let (second_cookie, _) = session_headers(&state).await;
        let app = router(state);
        let mut first = request(Method::GET, "/api/v1/downloads?path=", Body::empty());
        first.headers_mut().insert(
            header::COOKIE,
            HeaderValue::from_str(&first_cookie).unwrap(),
        );
        let first = json(app.clone().oneshot(first).await.unwrap()).await;
        let cursor = first["nextCursor"].as_str().unwrap();

        let mut foreign = request(
            Method::GET,
            &format!("/api/v1/downloads?path=&cursor={cursor}&page=1"),
            Body::empty(),
        );
        foreign.headers_mut().insert(
            header::COOKIE,
            HeaderValue::from_str(&second_cookie).unwrap(),
        );
        let response = app.oneshot(foreign).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(json(response).await["code"], "CURSOR_INVALID");
    }

    #[tokio::test]
    async fn directory_cursor_replays_lost_intermediate_and_final_pages_without_skipping() {
        let temp = tempfile::tempdir().unwrap();
        for index in 0..(DIRECTORY_PAGE_SIZE * 2 + 10) {
            fs::write(temp.path().join(format!("file-{index:04}.txt")), b"x").unwrap();
        }
        let state = test_state(temp.path());
        let (cookie, _) = session_headers(&state).await;
        let app = router(state);

        let mut first = request(Method::GET, "/api/v1/downloads?path=", Body::empty());
        first
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        let first = json(app.clone().oneshot(first).await.unwrap()).await;
        let cursor = first["nextCursor"].as_str().unwrap();
        let page = first["nextPage"].as_u64().unwrap();

        let page_uri = format!("/api/v1/downloads?path=&cursor={cursor}&page={page}");
        let mut next = request(Method::GET, &page_uri, Body::empty());
        next.headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        let next = json(app.clone().oneshot(next).await.unwrap()).await;
        assert_eq!(next["nextCursor"], cursor);
        assert_eq!(
            next["entries"].as_array().unwrap().len(),
            DIRECTORY_PAGE_SIZE
        );

        let mut retry = request(Method::GET, &page_uri, Body::empty());
        retry
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        let retry = json(app.clone().oneshot(retry).await.unwrap()).await;
        assert_eq!(
            retry, next,
            "a retry must replay the exact intermediate page"
        );

        let final_page = next["nextPage"].as_u64().unwrap();
        let final_uri = format!("/api/v1/downloads?path=&cursor={cursor}&page={final_page}");
        let mut final_request = request(Method::GET, &final_uri, Body::empty());
        final_request
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        let final_page = json(app.clone().oneshot(final_request).await.unwrap()).await;
        assert!(final_page["nextCursor"].is_null());
        assert_eq!(final_page["entries"].as_array().unwrap().len(), 10);

        let mut final_retry = request(Method::GET, &final_uri, Body::empty());
        final_retry
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        let final_retry = json(app.oneshot(final_retry).await.unwrap()).await;
        assert_eq!(
            final_retry, final_page,
            "a retry must replay the exact final page"
        );
    }

    #[tokio::test]
    async fn streams_ranges_as_forced_attachments() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("probe.txt"), b"0123456789").unwrap();
        let state = test_state(temp.path());
        let (cookie, _) = session_headers(&state).await;
        let app = router(state);
        let mut download = request(
            Method::GET,
            "/api/v1/download?path=probe.txt",
            Body::empty(),
        );
        download
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        download
            .headers_mut()
            .insert(header::RANGE, HeaderValue::from_static("bytes=2-5"));
        let response = app.clone().oneshot(download).await.unwrap();
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert!(response
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("attachment"));
        assert_eq!(
            response.headers().get(header::CONTENT_RANGE).unwrap(),
            "bytes 2-5/10"
        );
        assert_eq!(
            &to_bytes(response.into_body(), 1024).await.unwrap()[..],
            b"2345"
        );

        let mut head = request(
            Method::HEAD,
            "/api/v1/download?path=probe.txt",
            Body::empty(),
        );
        head.headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        head.headers_mut()
            .insert(header::RANGE, HeaderValue::from_static("bytes=7-"));
        let response = app.oneshot(head).await.unwrap();
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.headers().get(header::CONTENT_LENGTH).unwrap(), "3");
    }

    #[tokio::test]
    async fn dropping_unpolled_download_releases_active_registration() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("probe.bin"), vec![7_u8; 1024]).unwrap();
        let state = test_state(temp.path());
        let (cookie, _) = session_headers(&state).await;
        let app = router(state.clone());
        let mut download = request(
            Method::GET,
            "/api/v1/download?path=probe.bin",
            Body::empty(),
        );
        download
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        let response = app.oneshot(download).await.unwrap();
        assert_eq!(state.active_transfers().await, 1);
        drop(response);
        for _ in 0..20 {
            if state.active_transfers().await == 0 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(state.active_transfers().await, 0);
    }

    #[tokio::test]
    async fn upload_commit_remains_service_owned_after_the_waiter_is_dropped() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path());
        let id = Uuid::new_v4().to_string();
        let partial_path = state
            .partial_dir
            .as_ref()
            .unwrap()
            .join(format!("{id}.part"));
        let mut partial_file = create_upload_partial(&partial_path).unwrap();
        partial_file.write_all(b"x").unwrap();
        let transfer_id = state.record_transfer("upload", "late.txt", 1).await;
        let record = Arc::new(Mutex::new(UploadRecord {
            id: id.clone(),
            owner_session: "session".into(),
            owner_address: IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50)),
            name: "late.txt".into(),
            declared_size: 1,
            offset: 1,
            last_modified: 1,
            client_token: "late-token".into(),
            created_at: Instant::now(),
            last_activity: Instant::now(),
            cancelled: false,
            finalizing: true,
            cancel_signal: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            chunk_slots: Arc::new(tokio::sync::Semaphore::new(1)),
            partial_path,
            partial_file,
            transfer_id,
        }));
        state
            .uploads
            .lock()
            .await
            .insert(id.clone(), record.clone());
        let task_state = state.clone();
        let task_id = id.clone();
        let commit =
            tokio::spawn(async move { finalize_upload_owned(task_state, task_id, record).await });
        drop(commit);

        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let published = fs::read_dir(temp.path()).unwrap().flatten().any(|entry| {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    name.starts_with("late (") && name.ends_with(").txt")
                });
                if published && !state.uploads.lock().await.contains_key(&id) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("detaching the HTTP waiter must not detach the commit result");
    }

    #[tokio::test]
    async fn uploads_in_order_without_overwriting() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("foto.jpg"), b"vorhanden").unwrap();
        let state = test_state(temp.path());
        let (cookie, csrf) = session_headers(&state).await;
        let app = router(state);

        let mut create = request(
            Method::POST,
            "/api/v1/uploads",
            Body::from(r#"{"name":"foto.jpg","size":4,"lastModified":1}"#),
        );
        create.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        create
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        create
            .headers_mut()
            .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
        let created = json(app.clone().oneshot(create).await.unwrap()).await;
        let id = created["uploadId"].as_str().unwrap();

        let mut empty_chunk = request(
            Method::PATCH,
            &format!("/api/v1/uploads/{id}"),
            Body::empty(),
        );
        empty_chunk
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        empty_chunk
            .headers_mut()
            .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
        empty_chunk
            .headers_mut()
            .insert("upload-offset", HeaderValue::from_static("0"));
        let response = app.clone().oneshot(empty_chunk).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(json(response).await["code"], "EMPTY_CHUNK");

        let mut wrong_offset = request(
            Method::PATCH,
            &format!("/api/v1/uploads/{id}"),
            Body::from("test"),
        );
        wrong_offset
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        wrong_offset
            .headers_mut()
            .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
        wrong_offset
            .headers_mut()
            .insert("upload-offset", HeaderValue::from_static("1"));
        assert_eq!(
            app.clone().oneshot(wrong_offset).await.unwrap().status(),
            StatusCode::CONFLICT
        );

        let mut chunk = request(
            Method::PATCH,
            &format!("/api/v1/uploads/{id}"),
            Body::from("test"),
        );
        chunk
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        chunk
            .headers_mut()
            .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
        chunk
            .headers_mut()
            .insert("upload-offset", HeaderValue::from_static("0"));
        assert_eq!(
            app.clone().oneshot(chunk).await.unwrap().status(),
            StatusCode::OK
        );

        let mut complete = request(
            Method::POST,
            &format!("/api/v1/uploads/{id}/complete"),
            Body::empty(),
        );
        complete
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        complete
            .headers_mut()
            .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
        let completed = app.oneshot(complete).await.unwrap();
        assert_eq!(completed.status(), StatusCode::OK);
        let completed = json(completed).await;
        let saved_name = completed["name"].as_str().unwrap();
        assert_eq!(
            fs::read(temp.path().join("foto.jpg")).unwrap(),
            b"vorhanden"
        );
        assert_ne!(saved_name, "foto.jpg");
        assert_ne!(saved_name, "foto (2).jpg");
        assert_eq!(fs::read(temp.path().join(saved_name)).unwrap(), b"test");
    }

    #[tokio::test]
    async fn completes_zero_byte_upload() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path());
        let (cookie, csrf) = session_headers(&state).await;
        let app = router(state);
        let mut create = request(
            Method::POST,
            "/api/v1/uploads",
            Body::from(r#"{"name":"leer.txt","size":0,"lastModified":1}"#),
        );
        create.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        create
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        create
            .headers_mut()
            .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
        let created = json(app.clone().oneshot(create).await.unwrap()).await;
        let id = created["uploadId"].as_str().unwrap();

        let mut complete = request(
            Method::POST,
            &format!("/api/v1/uploads/{id}/complete"),
            Body::empty(),
        );
        complete
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        complete
            .headers_mut()
            .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
        let completed = app.oneshot(complete).await.unwrap();
        assert_eq!(completed.status(), StatusCode::OK);
        let completed = json(completed).await;
        let saved_name = completed["name"].as_str().unwrap();
        assert_ne!(saved_name, "leer.txt");
        assert_eq!(fs::metadata(temp.path().join(saved_name)).unwrap().len(), 0);
    }

    #[tokio::test]
    async fn completion_receipt_and_client_token_make_retries_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path());
        let (cookie, csrf) = session_headers(&state).await;
        let app = router(state.clone());
        let create_body = r#"{"name":"retry.bin","size":4,"lastModified":7,"clientToken":"0123456789abcdef0123456789abcdef"}"#;

        let mut create = request(Method::POST, "/api/v1/uploads", Body::from(create_body));
        create.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        create
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        create
            .headers_mut()
            .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
        let created = json(app.clone().oneshot(create).await.unwrap()).await;
        let id = created["uploadId"].as_str().unwrap().to_string();

        let mut chunk = request(
            Method::PATCH,
            &format!("/api/v1/uploads/{id}"),
            Body::from("data"),
        );
        chunk
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        chunk
            .headers_mut()
            .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
        chunk
            .headers_mut()
            .insert("upload-offset", HeaderValue::from_static("0"));
        assert_eq!(
            app.clone().oneshot(chunk).await.unwrap().status(),
            StatusCode::OK
        );

        let complete_request = || {
            let mut request = request(
                Method::POST,
                &format!("/api/v1/uploads/{id}/complete"),
                Body::empty(),
            );
            request
                .headers_mut()
                .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
            request
                .headers_mut()
                .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
            request
        };
        let first = json(app.clone().oneshot(complete_request()).await.unwrap()).await;
        let replay = json(app.clone().oneshot(complete_request()).await.unwrap()).await;
        assert_eq!(first["name"], replay["name"]);

        let mut recreate = request(Method::POST, "/api/v1/uploads", Body::from(create_body));
        recreate.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        recreate
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        recreate
            .headers_mut()
            .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
        let recreated = json(app.clone().oneshot(recreate).await.unwrap()).await;
        assert_eq!(recreated["uploadId"], id);
        assert_eq!(recreated["offset"], 4);

        let changed_address = Ipv4Addr::new(192, 168, 10, 51);
        let (changed_cookie, changed_csrf) = session_headers_for(&state, changed_address).await;
        let mut changed_recreate = request_from(
            Method::POST,
            "/api/v1/uploads",
            Body::from(create_body),
            changed_address,
        );
        changed_recreate.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        changed_recreate.headers_mut().insert(
            header::COOKIE,
            HeaderValue::from_str(&changed_cookie).unwrap(),
        );
        changed_recreate
            .headers_mut()
            .insert("x-dmdc-csrf", HeaderValue::from_str(&changed_csrf).unwrap());
        let changed_recreated = json(app.oneshot(changed_recreate).await.unwrap()).await;
        assert_eq!(changed_recreated["uploadId"], id);
        assert_eq!(changed_recreated["offset"], 4);
        assert_eq!(
            fs::read_dir(temp.path())
                .unwrap()
                .flatten()
                .filter(|entry| entry.file_name() != ".dmdc")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn rejects_tiny_nonfinal_upload_chunks() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path());
        let (cookie, csrf) = session_headers(&state).await;
        let app = router(state);
        let mut create = request(
            Method::POST,
            "/api/v1/uploads",
            Body::from(format!(
                r#"{{"name":"many-chunks.bin","size":{},"lastModified":1}}"#,
                CHUNK_SIZE + 1
            )),
        );
        create.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        create
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        create
            .headers_mut()
            .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
        let created = json(app.clone().oneshot(create).await.unwrap()).await;
        let id = created["uploadId"].as_str().unwrap();

        let mut chunk = request(
            Method::PATCH,
            &format!("/api/v1/uploads/{id}"),
            Body::from("x"),
        );
        chunk
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        chunk
            .headers_mut()
            .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
        chunk
            .headers_mut()
            .insert("upload-offset", HeaderValue::from_static("0"));
        let response = app.oneshot(chunk).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(json(response).await["code"], "CHUNK_SIZE_INVALID");
    }

    #[tokio::test]
    async fn limits_incomplete_uploads_per_client_address() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path());
        let (cookie, csrf) = session_headers(&state).await;
        let app = router(state);
        for index in 0..MAX_UPLOADS_PER_ADDRESS {
            let mut create = request(
                Method::POST,
                "/api/v1/uploads",
                Body::from(format!(
                    r#"{{"name":"datei-{index}.txt","size":0,"lastModified":1}}"#
                )),
            );
            create.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            create
                .headers_mut()
                .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
            create
                .headers_mut()
                .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
            assert_eq!(
                app.clone().oneshot(create).await.unwrap().status(),
                StatusCode::OK
            );
        }

        let mut rejected = request(
            Method::POST,
            "/api/v1/uploads",
            Body::from(r#"{"name":"zu-viel.txt","size":0,"lastModified":1}"#),
        );
        rejected.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        rejected
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        rejected
            .headers_mut()
            .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
        let response = app.oneshot(rejected).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(json(response).await["code"], "UPLOAD_CLIENT_LIMIT");
    }
}
