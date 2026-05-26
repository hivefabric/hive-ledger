# syntax=docker/dockerfile:1.7
#
# Honey Ledger — Postgres-backed event-sourced credit accounting service.
# Built from the workspace root (build context = ~/agent-cloud/hivefabric/)
# so the .dockerignore there keeps the host's target/ trees out.

FROM rust:1.89-bookworm AS builder
WORKDIR /workspace
COPY hive-ledger ./hive-ledger
WORKDIR /workspace/hive-ledger
RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -u 10004 -m -d /home/hive-ledger hive-ledger

COPY --from=builder /workspace/hive-ledger/target/release/hive-ledger /usr/local/bin/hive-ledger
COPY --from=builder /workspace/hive-ledger/migrations /etc/hive-ledger/migrations

EXPOSE 8100
USER hive-ledger
WORKDIR /home/hive-ledger

ENTRYPOINT ["/usr/local/bin/hive-ledger"]
