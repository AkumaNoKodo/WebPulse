use axum::{
    extract::{Path, State},
    response::Json,
    routing::get,
    Router,
};
use serde_json::json;

use crate::error::AppResult;
use crate::models::{CreateMonitor, Monitor, UpdateMonitor};
use crate::store::{self, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/monitors", get(list).post(create))
        .route("/monitors/{id}", get(read).put(update).delete(delete))
}

async fn list(State(state): State<AppState>) -> AppResult<Json<Vec<Monitor>>> {
    Ok(Json(store::list_monitors(&state).await?))
}

async fn read(State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Monitor>> {
    Ok(Json(store::get_monitor(&state, id).await?))
}

async fn create(
    State(state): State<AppState>,
    Json(payload): Json<CreateMonitor>,
) -> AppResult<Json<Monitor>> {
    Ok(Json(store::create_monitor(&state, payload).await?))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateMonitor>,
) -> AppResult<Json<Monitor>> {
    Ok(Json(store::update_monitor(&state, id, payload).await?))
}

async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    store::delete_monitor(&state, id).await?;

    Ok(Json(json!({ "deleted": true })))
}
