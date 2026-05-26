//! `hive-ledger` — Honey Ledger HTTP service entry point.
//!
//! Configure via env:
//!   LEDGER_BIND       bind address (default 0.0.0.0:8100)
//!   DATABASE_URL      Postgres URL (required)

use std::sync::Arc;

use hive_ledger::{router, AppState, PgLedger};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let bind = std::env::var("LEDGER_BIND").unwrap_or_else(|_| "0.0.0.0:8100".into());
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL must be set (e.g. postgres://user:pass@host:5432/db)")?;

    tracing::info!(%bind, "hive-ledger starting");
    let ledger = PgLedger::connect(&database_url).await?;
    ledger.migrate().await?;
    tracing::info!("migrations applied");

    let state = AppState {
        ledger: Arc::new(ledger),
    };
    let app = router(state);

    let listener = TcpListener::bind(&bind).await?;
    tracing::info!(%bind, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}
