use super::*;

#[derive(RustEmbed)]
#[folder = "../apps/mobile/dist/"]
pub(super) struct MobileAssets;

pub(super) async fn static_asset(uri: axum::http::Uri) -> Response {
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
