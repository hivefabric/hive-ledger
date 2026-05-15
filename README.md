# hive-ledger

> **STATUS: stub for Phase 2.** Empty scaffold; receives the Honey Ledger event-sourced credit accounting service when Phase 2 starts (~weeks 7–14 of the team plan in `agents.md`).

## What this will be

The L4 credit-accounting component of Honeycomb (per the canonical naming convention, "Honey Ledger" is a friendly alias; the technical name is **Ledger**). Event-sourced, double-entry, Postgres-backed with WAL replication and daily snapshots. Records every earn / spend / decay / clawback with the full multiplier set so any user can reproduce the credits charged for any task — non-negotiable #2 in the manifesto.

In private clusters (Tier 3), runs in audit-only mode. In Public Commons (Tier 1), runs the inflation controller per the spec formula.

## Where the design lives

- [`hivefabric/.github-private/docs/private/docs/02_architecture/09_honey_ledger.md`](https://github.com/hivefabric/.github-private/blob/main/docs/private/docs/02_architecture/09_honey_ledger.md) — component design.
- [`hivefabric/.github-private/docs/private/docs/04_finance/02_credit_economy_formulas.md`](https://github.com/hivefabric/.github-private/blob/main/docs/private/docs/04_finance/02_credit_economy_formulas.md) — every formula, multiplier, anti-gaming cap.
- [`hivefabric/.github-private/docs/private/agents.md`](https://github.com/hivefabric/.github-private/blob/main/docs/private/agents.md) §"Phase 1 — execution plan" → Phase 2 — for the build plan.

## Phase 2 deliverable

- Postgres schema (events, accounts, snapshots).
- WAL-replicated primary + at least one read replica.
- Append-only `audit.events` emission to the bus.
- Inflation controller as a nightly cron pinning `30-day total_earned ≈ total_spent × (1 - platform_take - avg_royalty)`.
- Decay job (1%/month after 90d inactivity; full forfeit at 24m).
- HTTP/Axum endpoints: `GET /api/credits/balance/{user_id}`, `GET /api/credits/history`, `POST /api/credits/clawback` (admin).

## License

Apache-2.0 — open worker, commercial gateway. The Ledger is gateway-side and proprietary in deployment, but the reference implementation in this repo (when authored) is open per the manifesto's commitment to transparency of the credit system.

