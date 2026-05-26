//! Credit-event endpoints. Every write goes through `record_event`; reads
//! are aggregations over the events table.

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::error::LedgerResult;
use crate::store::{CreditEvent, EventKind};
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/credits/debit", post(debit))
        .route("/v1/credits/refund", post(refund))
        .route("/v1/credits/credit", post(credit))
        .route("/v1/credits/reserve", post(reserve))
        .route("/v1/credits/release", post(release))
        .route("/v1/credits/:tenant_id/balance", get(balance))
        .route("/v1/credits/:tenant_id/events", get(events))
}

#[derive(Debug, Deserialize)]
struct EventRequest {
    tenant_id: Uuid,
    amount_credits: u64,
    #[serde(default)]
    correlation: Option<String>,
    #[serde(default)]
    idempotency_key: Option<String>,
    #[serde(default)]
    metadata: Option<Value>,
}

#[derive(Debug, Serialize)]
struct EventResponse {
    event: CreditEvent,
    /// Running balance after this event was applied. Convenience for callers
    /// that don't want to issue a follow-up GET.
    balance: i64,
}

async fn record(
    state: AppState,
    kind: EventKind,
    req: EventRequest,
) -> LedgerResult<Json<EventResponse>> {
    let metadata = req.metadata.unwrap_or_else(|| serde_json::json!({}));
    let event = state
        .ledger
        .record_event(
            req.tenant_id,
            kind,
            req.amount_credits,
            req.correlation.as_deref(),
            req.idempotency_key.as_deref(),
            metadata,
        )
        .await?;
    let balance = state.ledger.balance(req.tenant_id).await?;
    Ok(Json(EventResponse { event, balance }))
}

async fn debit(
    State(state): State<AppState>,
    Json(req): Json<EventRequest>,
) -> LedgerResult<Json<EventResponse>> {
    record(state, EventKind::Debit, req).await
}

async fn refund(
    State(state): State<AppState>,
    Json(req): Json<EventRequest>,
) -> LedgerResult<Json<EventResponse>> {
    record(state, EventKind::Refund, req).await
}

async fn credit(
    State(state): State<AppState>,
    Json(req): Json<EventRequest>,
) -> LedgerResult<Json<EventResponse>> {
    record(state, EventKind::Credit, req).await
}

async fn reserve(
    State(state): State<AppState>,
    Json(req): Json<EventRequest>,
) -> LedgerResult<Json<EventResponse>> {
    record(state, EventKind::Reservation, req).await
}

async fn release(
    State(state): State<AppState>,
    Json(req): Json<EventRequest>,
) -> LedgerResult<Json<EventResponse>> {
    record(state, EventKind::ReservationRelease, req).await
}

#[derive(Debug, Serialize)]
struct BalanceResponse {
    tenant_id: Uuid,
    balance: i64,
}

async fn balance(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
) -> LedgerResult<Json<BalanceResponse>> {
    let balance = state.ledger.balance(tenant_id).await?;
    Ok(Json(BalanceResponse { tenant_id, balance }))
}

#[derive(Debug, Deserialize, Default)]
struct EventsQuery {
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    50
}

async fn events(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    Query(q): Query<EventsQuery>,
) -> LedgerResult<Json<Vec<CreditEvent>>> {
    let events = state.ledger.events(tenant_id, q.limit).await?;
    Ok(Json(events))
}
