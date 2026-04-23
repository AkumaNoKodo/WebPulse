use axum::{
    extract::{Path, State},
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde_json::json;
use std::sync::Arc;

use crate::db::DbPool;
use crate::error::{AppError as AE, AppResult};
use crate::models::{CreateHeartbeat, Heartbeat, HeartbeatUpdate};

pub fn router(pool: DbPool) -> Router {
    Router::new()
        .route("/", get(list_heartbeats))
        .route("/", post(create_heartbeat))
        .route("/{id}", get(get_heartbeat))
        .route("/{id}", put(update_heartbeat))
        .route("/{id}", delete(delete_heartbeat))
        .route("/ping/{uuid}", post(ping_heartbeat))
        .with_state(Arc::new(pool))
}

async fn list_heartbeats(State(pool): State<Arc<DbPool>>) -> AppResult<Json<Vec<Heartbeat>>> {
    let heartbeats = sqlx::query_as::<_, Heartbeat>("SELECT * FROM heartbeats ORDER BY id")
        .fetch_all(&*pool)
        .await?;

    Ok(Json(heartbeats))
}

async fn get_heartbeat(
    Path(id): Path<i64>,
    State(pool): State<Arc<DbPool>>,
) -> AppResult<Json<Heartbeat>> {
    let heartbeat = sqlx::query_as::<_, Heartbeat>("SELECT * FROM heartbeats WHERE id = ?")
        .bind(id)
        .fetch_optional(&*pool)
        .await?
        .ok_or_else(|| AE::NotFound(format!("Heartbeat {} not found", id)))?;

    Ok(Json(heartbeat))
}

async fn create_heartbeat(
    State(pool): State<Arc<DbPool>>,
    Json(payload): Json<CreateHeartbeat>,
) -> AppResult<Json<Heartbeat>> {
    let uuid = crate::models::Heartbeat::generate_uuid();

    sqlx::query(
        r#"
        INSERT INTO heartbeats (uuid, name, expected_interval_secs, grace_period_secs)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(&uuid)
    .bind(&payload.name)
    .bind(payload.expected_interval_secs.unwrap_or(300))
    .bind(payload.grace_period_secs.unwrap_or(60))
    .execute(&*pool)
    .await?;

    let heartbeat = sqlx::query_as::<_, Heartbeat>("SELECT * FROM heartbeats WHERE uuid = ?")
        .bind(&uuid)
        .fetch_one(&*pool)
        .await?;

    Ok(Json(heartbeat))
}

async fn update_heartbeat(
    Path(id): Path<i64>,
    State(pool): State<Arc<DbPool>>,
    Json(payload): Json<HeartbeatUpdate>,
) -> AppResult<Json<Heartbeat>> {
    let existing = sqlx::query_as::<_, Heartbeat>("SELECT * FROM heartbeats WHERE id = ?")
        .bind(id)
        .fetch_optional(&*pool)
        .await?
        .ok_or_else(|| AE::NotFound(format!("Heartbeat {} not found", id)))?;

    let name = if let Some(ref n) = payload.name {
        n.clone()
    } else {
        existing.name.clone()
    };
    let expected_interval_secs = payload
        .expected_interval_secs
        .unwrap_or(existing.expected_interval_secs);
    let grace_period_secs = payload
        .grace_period_secs
        .unwrap_or(existing.grace_period_secs);

    sqlx::query(
        r#"
        UPDATE heartbeats 
        SET name = ?, expected_interval_secs = ?, grace_period_secs = ?, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(&name)
    .bind(expected_interval_secs)
    .bind(grace_period_secs)
    .bind(id)
    .execute(&*pool)
    .await?;

    let heartbeat = sqlx::query_as::<_, Heartbeat>("SELECT * FROM heartbeats WHERE id = ?")
        .bind(id)
        .fetch_one(&*pool)
        .await?;

    Ok(Json(heartbeat))
}

async fn delete_heartbeat(
    Path(id): Path<i64>,
    State(pool): State<Arc<DbPool>>,
) -> AppResult<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM heartbeats WHERE id = ?")
        .bind(id)
        .execute(&*pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AE::NotFound(format!("Heartbeat {} not found", id)));
    }

    Ok(Json(json!({"deleted": true})))
}

async fn ping_heartbeat(
    Path(uuid): Path<String>,
    State(pool): State<Arc<DbPool>>,
) -> AppResult<Json<serde_json::Value>> {
    let heartbeat = sqlx::query_as::<_, Heartbeat>("SELECT * FROM heartbeats WHERE uuid = ?")
        .bind(&uuid)
        .fetch_optional(&*pool)
        .await?
        .ok_or_else(|| AE::NotFound(format!("Heartbeat with uuid {} not found", uuid)))?;

    sqlx::query(
        r#"
        UPDATE heartbeats 
        SET status = 'healthy', last_ping_at = datetime('now'), updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(heartbeat.id)
    .execute(&*pool)
    .await?;

    Ok(Json(json!({"pinged": true, "id": heartbeat.id})))
}
