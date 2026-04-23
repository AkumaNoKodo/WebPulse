use axum::Router;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::broadcast;
use tracing::info;
use tracing_subscriber::EnvFilter;

use rustpulse::api::{heartbeat_router, monitor_router};
use rustpulse::config::Config;
use rustpulse::db::{create_pool, run_migrations, DbPool};
use rustpulse::services::scheduler::{MonitorEvent, Scheduler};
use rustpulse::sse;
use rustpulse::web;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("rustpulse=info".parse()?))
        .init();

    info!("Starting RustPulse...");

    let config = Config::from_file("config.toml")?;

    let pool = create_pool(&config.database).await?;
    run_migrations(&pool).await?;

    let (event_sender, _) = broadcast::channel::<MonitorEvent>(100);

    let scheduler = Arc::new(Scheduler::new(
        pool.clone(),
        config.scheduler.clone(),
        event_sender.clone(),
    ));

    let scheduler_clone = scheduler.clone();
    tokio::spawn(async move {
        scheduler_clone.start().await;
    });

    let app = Router::new()
        .nest(
            "/api",
            create_api_router(pool.clone(), event_sender.clone()),
        )
        .merge(web::web_router(pool.clone()));

    let addr = config.server.address();
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Shutting down...");
    scheduler.shutdown().await;

    pool.close().await;

    info!("Server stopped");
    Ok(())
}

fn create_api_router(pool: DbPool, event_sender: broadcast::Sender<MonitorEvent>) -> Router {
    Router::new()
        .nest("/monitors", monitor_router(pool.clone()))
        .nest("/heartbeats", heartbeat_router(pool.clone()))
        .nest("/sse", sse::router(pool, event_sender))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to listen for ctrl+c");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to listen for terminate")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received");
}
