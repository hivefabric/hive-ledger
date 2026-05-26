# CLAUDE.md — hive-ledger

## What this is

Honey Ledger — the event-sourced credit accounting service for HiveFabric tenants. Axum HTTP service on port 8100 (configurable via `LEDGER_BIND`). Called by hive-tenant-gateway before and after each `run_subagent` to reserve, debit, and refund credits. Every mutation appends an immutable row to `credit_events`; balances are derived by summing deltas. Idempotency keys prevent double-writes on retries.

## Key files

- `src/bin/hive_ledger.rs` — binary entry point; reads `DATABASE_URL`, runs migrations, starts Axum.
- `src/lib.rs` — `AppState`, `router()`, module declarations. Documents all HTTP endpoints.
- `src/store.rs` — `PgLedger`: all DB operations. `CreditEvent` is the canonical immutable row type. `EventKind`: `debit`, `refund`, `credit`, `reservation`, `reservation_release`.
- `src/routes/credits.rs` — all credit write/read endpoints.
- `src/routes/health.rs` — `GET /v1/healthz`.
- `migrations/20260520000001_init.sql` — creates `credit_events` table with idempotency key unique index.

## HTTP endpoints

```
POST /v1/credits/debit         record a debit (negative delta)
POST /v1/credits/refund        record a refund (positive delta)
POST /v1/credits/credit        out-of-band top-up (admin)
POST /v1/credits/reserve       reserve credits (negative delta, kind=reservation)
POST /v1/credits/release       release a prior reservation
GET  /v1/credits/{tenant}/balance
GET  /v1/credits/{tenant}/events
GET  /v1/healthz
```

All write endpoints accept an optional `idempotency_key`; a duplicate POST with the same key returns the original event id (no-op insert).

## How to run

```bash
# Requires Postgres
DATABASE_URL=postgres://hf:dev@localhost:5432/hf \
LEDGER_BIND=0.0.0.0:8100 \
cargo run --bin hive-ledger
```

Migrations run automatically at startup.

## How to test

```bash
# Unit tests (no DB)
cargo test -p hive-ledger

# Integration tests (require DATABASE_URL)
DATABASE_URL=postgres://hf:dev@localhost:5432/hf \
cargo test -p hive-ledger -- --ignored
```

## Architecture notes

- Append-only: `credit_events` rows are never updated or deleted. Balances are always derived by `SUM(delta_credits)` per tenant. This is the audit guarantee.
- Idempotency: `idempotency_key` has a unique index per tenant. Callers (tenant-gateway) should use the task_id as the idempotency key to survive retries.
- No authentication on the HTTP surface today — it is assumed to be internal-only (not exposed outside the cluster). Production deployments must network-isolate it.
- `hive-ledger` has no dependency on `hive-sdk`; it is intentionally self-contained to keep the audit surface small.

### Key env vars

| Var | Default | Purpose |
|---|---|---|
| `DATABASE_URL` | — | Required. Postgres DSN. |
| `LEDGER_BIND` | `0.0.0.0:8100` | Bind address. |

## What's not done

- STATUS: Phase 2 — the core append/query loop is implemented and tested, but several Phase 2 deliverables are not yet done:
  - Inflation controller (nightly cron, `30-day total_earned ≈ total_spent × (1 - take - royalty)`).
  - Decay job (1%/month after 90d inactivity; full forfeit at 24m).
  - WAL replication + read replica setup.
  - Admin `POST /v1/credits/clawback` endpoint.
- No HTTP auth layer — assumes network isolation.
- Reserve/release cycle exists but the gateway does not yet enforce reserve-before-dispatch end-to-end.
