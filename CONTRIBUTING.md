# Contributing

Bug reports and pull requests are welcome. Use the issue templates for ordinary bugs and feature requests; report vulnerabilities through [SECURITY.md](SECURITY.md).

## Development

Use Node 24, the Rust version in `rust-toolchain.toml`, Go from `services/api/go.mod`, and Python 3.9+. macOS requires Xcode Command Line Tools. Windows requires Visual Studio C++ Build Tools, Windows SDK and WebView2. See the [README](README.md) for native packaging commands.

```sh
npm ci
npm run build
cargo fmt --all --check
cargo test --locked -p dengex-agent-core -p dengex-transport --lib
cargo clippy --locked -p dengex-agent-core -p dengex-transport -p dengex-platform-windows -- -D warnings
# In services/api:
go test -race ./...
```

`bash scripts/check.sh` runs the common local checks. Native Windows DLL tests need an x64 Native Tools shell; TURN and deployment tests need Docker. CI runs these separately. The server is deliberately memory-only; no database setup is required for the attended beta.

Keep changes small and explain the user-visible behavior and validation. Preserve explicit local consent, native lease expiry, fail-closed behavior, bounded queues and stop controls. Never add silent capture, credential bypass, permanent access by default, or success indicators without measured evidence. Do not commit generated binaries, actual endpoints, tokens, screen content or machine-specific paths.

Run meaningful tests for changes to authorization, native capture, transport and packaging. An automated build is not a successful real-device capture test; record the distinction. Avoid modifying historical evidence files to imply tests were rerun.

Contributions are made under the repository's MIT license. Third-party dependencies keep their own licenses. Update lockfiles and ensure packaged license notices remain available when changing dependencies.
