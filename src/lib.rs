//! Honey Ledger — Postgres-backed event-sourced credit accounting.
//!
//! HTTP service. Every mutation writes one immutable row to
//! `credit_events`; balances are derived by summing the deltas for a
//! tenant. The tenant gateway calls this service before/after each
//! `run_subagent` to reserve, debit, and refund credits per task.
//!
//! Endpoints:
//!   POST /v1/credits/debit         — record a debit (negative delta)
//!   POST /v1/credits/refund        — record a refund (positive delta)
//!   POST /v1/credits/credit        — record an out-of-band top-up (admin)
//!   POST /v1/credits/reserve       — reserve credits (negative delta)
//!   POST /v1/credits/release       — release a prior reservation
//!   GET  /v1/credits/{tenant}/balance
//!   GET  /v1/credits/{tenant}/events
//!   GET  /v1/healthz
//!
//! All write endpoints accept an `idempotency_key`; a duplicate POST with
//! the same key is a no-op (returns the original event id).

pub mod error;
pub mod routes;
pub mod store;

pub use error::{LedgerError, LedgerResult};
pub use store::PgLedger;

use std::sync::Arc;

use axum::Router;

#[derive(Clone)]
pub struct AppState {
    pub ledger: Arc<PgLedger>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(routes::credits::router())
        .merge(routes::health::router())
        .with_state(state)
}
