# hive-ledger

Event-sourced credit accounting service for HiveFabric. Records every earn/spend/refund as an immutable append-only event; derives balances by summing deltas. Idempotency keys prevent double-writes on retries.

Runs on port 8100. Called by `hive-tenant-gateway` to debit before dispatch and refund on failure.

---

## HTTP endpoints

```
POST /v1/credits/debit         record a debit (tenant spends credits)
POST /v1/credits/refund        record a refund (reverse a prior debit)
POST /v1/credits/credit        out-of-band top-up (admin use)
POST /v1/credits/reserve       reserve credits before dispatch
POST /v1/credits/release       release a prior reservation
GET  /v1/credits/{tenant_id}/balance
GET  /v1/credits/{tenant_id}/events?limit=50
GET  /v1/healthz
```

All write endpoints accept `idempotency_key` — duplicate POSTs with the same key are no-ops that return the original event.

### Request shape (write endpoints)

```json
{
  "tenant_id": "uuid",
  "amount_credits": 1,
  "correlation": "task-abc123",
  "idempotency_key": "debit:task-abc123",
  "metadata": { "capability_urn": "oasf://commons/inference/qwen2.5-7b/v1" }
}
```

---

## How to run

```bash
DATABASE_URL=postgres://hf:dev@localhost:5432/hf \
LEDGER_BIND=0.0.0.0:8100 \
cargo run --bin hive-ledger
```

Migrations run automatically at startup.

## Docker

```bash
docker build -t hivefabric/hive-ledger .
docker run -e DATABASE_URL=postgres://... -p 8100:8100 hivefabric/hive-ledger
```

---

## Architecture

- **Append-only**: `credit_events` rows are never updated or deleted. Balances derived by `SUM(delta_credits) WHERE tenant_id = ?`. Any tenant can verify their balance from first principles.
- **No HTTP auth**: Assumes network isolation — internal service only. Do not expose port 8100 outside the cluster.
- **Self-contained**: No dependency on `hive-sdk`. Keeps the audit surface minimal.
- **Idempotency**: `UNIQUE(tenant_id, idempotency_key)` prevents double-billing on retries.

---

## Environment variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `DATABASE_URL` | Yes | — | Postgres DSN |
| `LEDGER_BIND` | No | `0.0.0.0:8100` | Bind address |

---

## Testing

```bash
cargo test                         # unit tests, no DB needed
DATABASE_URL_TEST=postgres://... cargo test -- --ignored  # integration tests
```

---

## What's not yet implemented

- Inflation controller (nightly cron)
- Credit decay job (1%/month after 90d inactivity)
- WAL replication / read replica
- Admin clawback endpoint
- HTTP auth (currently assumes internal network isolation)
- Reserve-before-dispatch end-to-end enforcement in tenant-gateway
