use super::*;

pub(super) fn client_ip(connect: ConnectInfo<SocketAddr>) -> IpAddr {
    connect.0.ip()
}

pub(super) fn cookie_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix("ldtg_session="))
}

pub(super) async fn authorized(
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
            .get("x-ldtg-csrf")
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

pub(super) async fn ensure_session_active(
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
#[serde(rename_all = "camelCase")]
pub(super) struct AuthRequest {
    code: String,
    device_name: Option<String>,
}

pub(super) async fn auth(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    security: Option<Extension<Arc<ConnectionSecurity>>>,
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
    let peer_key = security
        .map(|Extension(security)| security.peer_key().to_owned())
        .unwrap_or_else(|| format!("ip:{address}"));
    match state.verify_access_code_for_peer(&peer_key, &payload.code) {
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
        .unwrap_or("Mobiler Browser");
    let session = state
        .create_named_session(address, user_agent, payload.device_name.as_deref())
        .await
        .map_err(|limit| match limit {
            SessionCreateError::InvalidDeviceName => ApiFailure::new(
                StatusCode::BAD_REQUEST,
                "DEVICE_NAME_INVALID",
                format!(
                    "Der Gerätename darf höchstens {MAX_DEVICE_NAME_CHARS} Zeichen enthalten und keine Steuerzeichen verwenden."
                ),
            ),
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
            "ldtg_session={}; Path=/; HttpOnly; SameSite=Strict",
            session.token
        ))
        .expect("generated cookie is valid"),
    );
    Ok(response)
}

pub(super) async fn session(
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

pub(super) async fn logout(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let session = authorized(&state, &headers, client.ip(), true).await?;
    state.revoke_session(&session.id).await;
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static("ldtg_session=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0"),
    );
    Ok(response)
}
