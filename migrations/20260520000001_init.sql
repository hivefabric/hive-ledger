-- Honey Ledger — initial schema.
--
-- Event-sourced: balances are derived by summing the events for a tenant.
-- Every operation (debit, refund, credit, reservation, reservation-release)
-- writes one row and never edits prior rows. Reconciliation jobs and
-- usage reports both read this same table; nothing else mutates it.

CREATE TABLE IF NOT EXISTS credit_events (
    id              UUID         PRIMARY KEY,
    tenant_id       UUID         NOT NULL,
    -- `debit` | `refund` | `credit` | `reservation` | `reservation_release`
    kind            TEXT         NOT NULL,
    -- Signed delta applied to the tenant's running balance. debit/reservation
    -- are negative; refund/credit/reservation_release are positive.
    -- Negative reservations are written at reserve time; reservation_release
    -- writes the matching positive entry when the reservation expires or is
    -- explicitly released.
    delta_credits   BIGINT       NOT NULL,
    -- Free-form correlation id supplied by the caller; usually a task_id.
    -- `idempotency_key` lets a duplicate POST be a no-op.
    correlation     TEXT,
    idempotency_key TEXT         UNIQUE,
    -- Caller-supplied context, e.g. {"capability_urn": "...", "model": "..."}.
    metadata        JSONB        NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_credit_events_tenant_created
    ON credit_events (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_credit_events_correlation
    ON credit_events (tenant_id, correlation)
    WHERE correlation IS NOT NULL;
