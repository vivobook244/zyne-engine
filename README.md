# ZYNE Engine

**BEYOND THE FRAME.**

ZYNE (ZYNE Yields Native Experiences) is an experimental Rust-powered 3D game engine focused on building complete, playable third-person games.

The initial visual target is the atmosphere and gameplay presentation of classic survival-action games from the sixth console generation. Windows is the first platform; Linux support follows.

## Goals

- Build a practical game runtime, not only a rendering demo.
- Keep the engine modular in one Cargo workspace.
- Start with forward rendering and Blinn–Phong lighting.
- Provide a third-person / over-the-shoulder gameplay foundation.
- Deliver a runnable vertical slice at every major milestone.

## Status

**Phase 0 — Project Foundation** is in progress. See the [roadmap](docs/ROADMAP.md), [Phase 0 tracker](docs/development/PHASE-0-FOUNDATION.md), [architecture overview](docs/architecture/OVERVIEW.md), [math conventions](docs/architecture/MATH.md), [configuration reference](docs/CONFIGURATION.md), and [versioning policy](docs/VERSIONING.md).

## Development

Requirements: Rust stable toolchain with Cargo.

~~~sh
rustup update stable
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace
cargo run -p zyne-runtime --example headless_smoke
~~~

## Principles

- Rust stable.
- One repository and one Cargo workspace.
- No Docker or microservice architecture.
- External crates are subsystem building blocks, not a replacement for ZYNE's public API.
- Prefer testable vertical slices over premature abstraction.

## License

MIT. See [LICENSE](LICENSE).
