use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Template error: {0}")]
    Template(#[from] askama::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    fn status(&self) -> StatusCode {
        match self {
            AppError::Database(_) | AppError::Template(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Request(_) => StatusCode::BAD_GATEWAY,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.status(), Json(json!({ "error": self.to_string() }))).into_response()
    }
}

/// htmx 4 swaps error responses into the page, so the web layer must answer with
/// a renderable fragment instead of the JSON body `AppError` produces for the API.
#[derive(Debug)]
pub struct WebError(AppError);

impl<E: Into<AppError>> From<E> for WebError {
    fn from(error: E) -> Self {
        WebError(error.into())
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let message = escape_html(&self.0.to_string());
        let body = format!(
            r#"<div class="border border-red-700 text-red-400 rounded p-4 text-sm">{message}</div>"#
        );

        (self.0.status(), Html(body)).into_response()
    }
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
