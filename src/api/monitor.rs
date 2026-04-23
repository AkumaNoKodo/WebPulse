use axum::{
    extract::{Path, State},
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde_json::json;
use std::sync::Arc;

use crate::db::DbPool;
use crate::error::AppError as AE;
use crate::models::{CreateMonitor, Monitor, UpdateMonitor};

pub fn router(pool: DbPool) -> Router {
    let state = Arc::new(pool);

    Router::new()
        .route("/", get(list_monitors))
        .route("/", post(create_monitor))
        .route("/{id}", get(get_monitor))
        .route("/{id}", put(update_monitor))
        .route("/{id}", delete(delete_monitor))
        .with_state(state)
}

async fn list_monitors(State(pool): State<Arc<DbPool>>) -> Result<Json<Vec<Monitor>>, AE> {
    let monitors = sqlx::query_as::<_, Monitor>("SELECT * FROM monitors ORDER BY id")
        .fetch_all(&*pool)
        .await
        .map_err(AE::from)?;

    Ok(Json(monitors))
}

async fn get_monitor(
    Path(id): Path<i64>,
    State(pool): State<Arc<DbPool>>,
) -> Result<Json<Monitor>, AE> {
    let monitor = sqlx::query_as::<_, Monitor>("SELECT * FROM monitors WHERE id = ?")
        .bind(id)
        .fetch_optional(&*pool)
        .await
        .map_err(AE::from)?
        .ok_or_else(|| AE::NotFound(format!("Monitor {} not found", id)))?;

    Ok(Json(monitor))
}

async fn create_monitor(
    State(pool): State<Arc<DbPool>>,
    Json(payload): Json<CreateMonitor>,
) -> Result<Json<Monitor>, AE> {
    let result = sqlx::query(
        r#"
        INSERT INTO monitors (name, url, check_interval_secs, timeout_secs)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(&payload.name)
    .bind(&payload.url)
    .bind(payload.check_interval_secs.unwrap_or(60))
    .bind(payload.timeout_secs.unwrap_or(30))
    .execute(&*pool)
    .await
    .map_err(AE::from)?;

    let id = result.last_insert_rowid();

    let monitor = sqlx::query_as::<_, Monitor>("SELECT * FROM monitors WHERE id = ?")
        .bind(id)
        .fetch_one(&*pool)
        .await
        .map_err(AE::from)?;

    Ok(Json(monitor))
}

async fn update_monitor(
    Path(id): Path<i64>,
    State(pool): State<Arc<DbPool>>,
    Json(payload): Json<UpdateMonitor>,
) -> Result<Json<Monitor>, AE> {
    let existing = sqlx::query_as::<_, Monitor>("SELECT * FROM monitors WHERE id = ?")
        .bind(id)
        .fetch_optional(&*pool)
        .await
        .map_err(AE::from)?
        .ok_or_else(|| AE::NotFound(format!("Monitor {} not found", id)))?;

    let name = if let Some(ref n) = payload.name {
        n.clone()
    } else {
        existing.name.clone()
    };
    let url = if let Some(ref u) = payload.url {
        u.clone()
    } else {
        existing.url.clone()
    };
    let check_interval_secs = payload
        .check_interval_secs
        .unwrap_or(existing.check_interval_secs);
    let timeout_secs = payload.timeout_secs.unwrap_or(existing.timeout_secs);

    sqlx::query(
        r#"
        UPDATE monitors 
        SET name = ?, url = ?, check_interval_secs = ?, timeout_secs = ?, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(&name)
    .bind(&url)
    .bind(check_interval_secs)
    .bind(timeout_secs)
    .bind(id)
    .execute(&*pool)
    .await
    .map_err(AE::from)?;

    let monitor = sqlx::query_as::<_, Monitor>("SELECT * FROM monitors WHERE id = ?")
        .bind(id)
        .fetch_one(&*pool)
        .await
        .map_err(AE::from)?;

    Ok(Json(monitor))
}

async fn delete_monitor(
    Path(id): Path<i64>,
    State(pool): State<Arc<DbPool>>,
) -> Result<Json<serde_json::Value>, AE> {
    let result = sqlx::query("DELETE FROM monitors WHERE id = ?")
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(AE::from)?;

    if result.rows_affected() == 0 {
        return Err(AE::NotFound(format!("Monitor {} not found", id)));
    }

    Ok(Json(json!({"deleted": true})))
}
