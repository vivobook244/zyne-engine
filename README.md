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

**Phase 5.5.4 — Material Texture Integration** is the current implementation milestone. CPU texture validation, glTF image normalization, runtime resolution, GPU upload/binding, and cache lifecycle reconciliation are implemented. The material showcase has been visually verified on Intel UHD Graphics 610: blue ceramic and amber diffuse textures render, checker fallback remains visible, and viewport resizing preserves the textures. Headless GPU CI verifies sRGB/Linear allocation, renderer-compatible bind-group creation, and exact pixel readback on Ubuntu and Windows. Production-path cache synchronization tests cover deduplication, revision reuse/replacement, shared-reference lifetime, pruning, and failed-replacement safety. Binding-selection tests also verify material texture versus checker fallback on both CI platforms; a full per-draw frame-render integration test and final physical-GPU regression review remain open. See the [roadmap](docs/ROADMAP.md), [Phase 5.5.4 tracker](docs/development/PHASE-5-5-4-MATERIAL-TEXTURE-INTEGRATION.md), [architecture overview](docs/architecture/OVERVIEW.md), [lighting guide](docs/architecture/LIGHTING.md), [math conventions](docs/architecture/MATH.md), [configuration reference](docs/CONFIGURATION.md), and [versioning policy](docs/VERSIONING.md).

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
