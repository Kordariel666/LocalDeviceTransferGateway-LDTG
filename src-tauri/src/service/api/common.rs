use super::*;

#[derive(Debug)]
pub(super) struct ApiFailure {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiFailure {
    pub(super) fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
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

pub(super) type ApiResult<T> = Result<T, ApiFailure>;

pub(super) async fn method_not_allowed() -> ApiFailure {
    ApiFailure::new(
        StatusCode::METHOD_NOT_ALLOWED,
        "METHOD_NOT_ALLOWED",
        "Diese Aktion wird von DMDC nicht angeboten.",
    )
}

pub(super) async fn request_guard(
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

pub(super) fn secure_response(mut response: Response) -> Response {
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
