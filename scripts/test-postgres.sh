#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
docker compose -f infra/compose.yaml exec -T postgres psql -U dengex_migrator -d dengex < scripts/test-postgres.sql
