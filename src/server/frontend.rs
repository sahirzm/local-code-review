use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct Frontend;

pub async fn serve_embedded(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let candidate = if path.is_empty() { "index.html" } else { path };

    if let Some(resp) = lookup(candidate) {
        return resp;
    }

    // SPA fallback: serve index.html for unknown paths so client-side routing works.
    if let Some(resp) = lookup("index.html") {
        return resp;
    }

    (StatusCode::NOT_FOUND, "Frontend assets not embedded").into_response()
}

fn lookup(path: &str) -> Option<Response> {
    let asset = Frontend::get(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    Some(
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(Body::from(asset.data.into_owned()))
            .unwrap(),
    )
}
