use axum::Router;
use std::sync::Arc;
use tokio::signal;
use tracing::info;
use tracing_subscriber::EnvFilter;

use rustpulse::api::{heartbeat_router, monitor_router};
use rustpulse::config::Config;
use rustpulse::db::{create_pool, run_migrations, DbPool};
use rustpulse::services::scheduler::Scheduler;
use rustpulse::web;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_path = std::env::var("CONFIG_FILE").unwrap_or_else(|_| "config.toml".to_string());
    let config = Config::from_file(&config_path)?;

    let log_directive = format!("rustpulse={}", config.logging.level);
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(log_directive.parse()?))
        .init();

    info!("Starting RustPulse...");

    let pool = create_pool(&config.database).await?;
    run_migrations(&pool).await?;

    let scheduler = Arc::new(Scheduler::new(pool.clone(), config.scheduler.clone()));

    let scheduler_clone = scheduler.clone();
    tokio::spawn(async move {
        scheduler_clone.start().await;
    });

    let app = Router::new()
        .nest("/api", create_api_router(pool.clone()))
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

fn create_api_router(pool: DbPool) -> Router {
    Router::new()
        .nest("/monitors", monitor_router(pool.clone()))
        .nest("/heartbeats", heartbeat_router(pool))
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
