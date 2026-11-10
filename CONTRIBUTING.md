# Contributing to ZYNE

## Before changing code

1. Read the architecture overview and roadmap.
2. Keep changes in the crate that owns the responsibility.
3. Explain why a dependency is needed before introducing it.
4. Avoid circular dependencies and leaking implementation details through public APIs.

## Local validation

~~~sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace
~~~

## Rust conventions

- Use stable Rust and the workspace edition.
- Keep unsafe code forbidden unless a reviewed exception is approved.
- Return typed errors for recoverable failures; do not silently discard errors.
- Prefer focused modules and document public APIs/invariants.

## Change scope

Each phase should deliver a working vertical slice and update its acceptance checklist. Do not add speculative systems just to reserve a name or directory.
