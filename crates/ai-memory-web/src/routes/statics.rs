//! Static asset handlers: Tailwind CSS and logo images.
//!
//! Assets are embedded at compile time so the binary is fully
//! self-contained — no runtime file access needed.

use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::IntoResponse;

/// Compiled Tailwind CSS. The path is emitted by `build.rs` via the
/// `AI_MEMORY_WEB_TAILWIND_CSS` cargo env var.
static TAILWIND_CSS: &str = include_str!(env!("AI_MEMORY_WEB_TAILWIND_CSS"));

/// Light-mode logo (PNG).
static LOGO_LIGHT: &[u8] = include_bytes!("../../../../docs/logo.png");

/// Dark-mode logo (PNG).
static LOGO_DARK: &[u8] = include_bytes!("../../../../docs/logo-dark.png");

/// `GET /static/tailwind.css`
pub(crate) async fn tailwind_css() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/css; charset=utf-8"),
    );
    (StatusCode::OK, headers, TAILWIND_CSS)
}

/// `GET /static/logo.png`
pub(crate) async fn logo_light() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
    (StatusCode::OK, headers, LOGO_LIGHT)
}

/// `GET /static/logo-dark.png`
pub(crate) async fn logo_dark() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
    (StatusCode::OK, headers, LOGO_DARK)
}
