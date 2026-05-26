//! Postgres-backed ledger store. Every mutation goes through
//! [`PgLedger::record_event`]; reads are aggregations over `credit_events`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;
use uuid::Uuid;

use crate::error::{LedgerError, LedgerResult};

/// One immutable row in `credit_events`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditEvent {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub kind: String,
    pub delta_credits: i64,
    pub correlation: Option<String>,
    pub idempotency_key: Option<String>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Debit,
    Refund,
    Credit,
    Reservation,
    ReservationRelease,
}

impl EventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Debit => "debit",
            Self::Refund => "refund",
            Self::Credit => "credit",
            Self::Reservation => "reservation",
            Self::ReservationRelease => "reservation_release",
        }
    }

    /// Sign convention enforced at write time. Debit and reservation lower
    /// the balance; refund, credit, and release raise it.
    pub fn expected_sign(&self) -> i8 {
        match self {
            Self::Debit | Self::Reservation => -1,
            Self::Refund | Self::Credit | Self::ReservationRelease => 1,
        }
    }
}

#[derive(Clone)]
pub struct PgLedger {
    pool: PgPool,
}

impl PgLedger {
    pub async fn connect(database_url: &str) -> LedgerResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> LedgerResult<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    /// Record one event. `amount_credits` is unsigned magnitude; the kind
    /// dictates the sign that's persisted. Idempotency keys make duplicate
    /// POSTs a no-op (returns the original event row).
    pub async fn record_event(
        &self,
        tenant_id: Uuid,
        kind: EventKind,
        amount_credits: u64,
        correlation: Option<&str>,
        idempotency_key: Option<&str>,
        metadata: Value,
    ) -> LedgerResult<CreditEvent> {
        if amount_credits == 0 {
            return Err(LedgerError::Invalid("amount_credits must be > 0".into()));
        }
        let signed = kind.expected_sign() as i64 * amount_credits as i64;

        // Idempotency: if a key was supplied and a row already exists with it,
        // return that row instead of inserting again.
        if let Some(key) = idempotency_key {
            if let Some(existing) = self.find_by_idempotency_key(key).await? {
                return Ok(existing);
            }
        }

        let id = Uuid::new_v4();
        let row = sqlx::query(
            r#"
            INSERT INTO credit_events (id, tenant_id, kind, delta_credits, correlation, idempotency_key, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, tenant_id, kind, delta_credits, correlation, idempotency_key, metadata, created_at
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(kind.as_str())
        .bind(signed)
        .bind(correlation)
        .bind(idempotency_key)
        .bind(&metadata)
        .fetch_one(&self.pool)
        .await?;
        Ok(row_to_event(&row))
    }

    pub async fn find_by_idempotency_key(
        &self,
        key: &str,
    ) -> LedgerResult<Option<CreditEvent>> {
        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, kind, delta_credits, correlation, idempotency_key, metadata, created_at
            FROM credit_events
            WHERE idempotency_key = $1
            "#,
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.as_ref().map(row_to_event))
    }

    /// Sum of all deltas for a tenant.
    pub async fn balance(&self, tenant_id: Uuid) -> LedgerResult<i64> {
        let row = sqlx::query(
            r#"SELECT COALESCE(SUM(delta_credits)::bigint, 0) AS bal FROM credit_events WHERE tenant_id = $1"#,
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.get::<i64, _>("bal"))
    }

    pub async fn events(
        &self,
        tenant_id: Uuid,
        limit: i64,
    ) -> LedgerResult<Vec<CreditEvent>> {
        let rows = sqlx::query(
            r#"
            SELECT id, tenant_id, kind, delta_credits, correlation, idempotency_key, metadata, created_at
            FROM credit_events
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(tenant_id)
        .bind(limit.clamp(1, 1000))
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(row_to_event).collect())
    }
}

fn row_to_event(row: &sqlx::postgres::PgRow) -> CreditEvent {
    CreditEvent {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        kind: row.get::<String, _>("kind"),
        delta_credits: row.get("delta_credits"),
        correlation: row.get("correlation"),
        idempotency_key: row.get("idempotency_key"),
        metadata: row.get("metadata"),
        created_at: row.get("created_at"),
    }
}
