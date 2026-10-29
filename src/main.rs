use movie_streamer::{api, config::Config, error::AppError};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let config = Config::from_env().unwrap_or_else(|e| {
        eprintln!("configuration error: {e}");
        std::process::exit(1);
    });

    init_tracing(&config);

    if config.api_token.is_none() {
        tracing::warn!("APP_API_TOKEN is not configured; API is currently unprotected");
    }

    let addr = config.bind_addr().unwrap_or_else(|e| {
        eprintln!("configuration error: {e}");
        std::process::exit(1);
    });

    let app = api::router(config);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| AppError::internal(format!("bind {addr}: {e}")))?;

    tracing::info!(%addr, version = env!("CARGO_PKG_VERSION"), "server started");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| AppError::internal(format!("server error: {e}")))?;

    tracing::info!("server stopped");
    Ok(())
}

fn init_tracing(config: &Config) {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(config.rust_log.as_str()));

    tracing_subscriber::fmt().with_env_filter(filter).init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received");
}
