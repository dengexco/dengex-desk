#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all --check
cargo test --locked -p dengex-agent-core
cargo clippy --locked -p dengex-agent-core -p dengex-transport -p dengex-platform-windows -- -D warnings
(cd services/api && go test -race ./...)
npm ci
npm run build
npm audit --audit-level=high
