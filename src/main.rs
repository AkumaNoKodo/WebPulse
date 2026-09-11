use axum::Router;
use std::time::Duration;
use tokio::signal;
use tracing::info;
use tracing_subscriber::EnvFilter;

use webpulse::config::Config;
use webpulse::db::{create_pool, run_migrations};
use webpulse::services::scheduler;
use webpulse::store::AppState;
use webpulse::{api, web};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_path = std::env::var("CONFIG_FILE").unwrap_or_else(|_| "config.toml".to_string());
    let config = Config::from_file(&config_path)?;

    if std::env::args().nth(1).as_deref() == Some("--health") {
        return health_check(config.server.port).await;
    }

    let log_directive = format!("webpulse={}", config.logging.level);
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(log_directive.parse()?))
        .init();

    info!("Starting WebPulse...");

    let pool = create_pool(&config.database).await?;
    run_migrations(&pool).await?;

    let state = AppState {
        pool: pool.clone(),
        monitor: config.monitor.clone(),
        http: reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .build()?,
    };

    tokio::spawn(scheduler::run(state.clone(), config.scheduler.clone()));

    let app = Router::new()
        .nest("/api", api::router())
        .merge(web::router())
        .with_state(state);

    let listener =
        tokio::net::TcpListener::bind((config.server.host.as_str(), config.server.port)).await?;
    info!("Listening on {}", listener.local_addr()?);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Shutting down...");
    pool.close().await;

    Ok(())
}

async fn health_check(port: u16) -> anyhow::Result<()> {
    let response = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/"))
        .timeout(Duration::from_secs(3))
        .send()
        .await?;

    anyhow::ensure!(
        response.status().is_success(),
        "health check failed: HTTP {}",
        response.status()
    );

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("ctrl+c handler must be installable");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("SIGTERM handler must be installable")
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
