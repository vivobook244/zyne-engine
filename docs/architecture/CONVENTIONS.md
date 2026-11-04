# Engine Conventions

This document records conventions that must remain consistent across engine crates.

## Coordinate and math conventions

The authoritative mathematical contract is [Math and Coordinate Conventions](MATH.md). In short: right-handed world, +Y up, +Z engine forward, column-vector algebra, column-major matrices, and local transform composition `T * R * S`. Camera view space looks down -Z and uses a [0, 1] projection depth range for wgpu. GLTF conversion belongs at the importer boundary.

## API and module policy

- Keep crate roots small; implementation belongs in focused modules.
- Expose stable crate APIs through deliberate `pub use` re-exports.
- Keep implementation modules private unless external consumers have a reason to access them.
- Public types and methods must have Rustdoc comments.
- Recoverable failures use `ZyneResult<T>` and typed `ZyneError` variants.
- Avoid panics for invalid project input or normal runtime failures.
- Do not expose third-party subsystem types through core contracts unless that coupling is intentional and documented.
- Before 0.1, public APIs may evolve, but changes must be intentional, documented, and accompanied by migration notes when they affect consumers.
- After 0.1, compatibility is evaluated against the documented versioning policy; breaking changes require an explicit minor/major-version decision rather than accidental re-exports.

## Logging policy

- Engine libraries emit structured events through `tracing`.
- Libraries must not initialize a process-wide subscriber.
- The executable/game host initializes diagnostics once, at startup.
- Default filter is `info`; `RUST_LOG` may override it.
- Use trace/debug for high-volume details, info for lifecycle milestones, warn for recoverable degraded behavior, and error for failed operations.
- Runtime startup and shutdown should emit one lifecycle event each; subsystem crates emit their own ready/failed events when those subsystems exist.

## Resource policy

- Persisted assets and scene references use `zyne_core::ResourceId` (UUID v4).
- Runtime references use typed `Handle<T>` values containing a slot index and generation.
- Persistent IDs survive project/scene serialization; runtime handles do not.
- Registries must validate both handle index and generation. Releasing a slot invalidates its previous handles before reuse.
- Resource managers own loaded runtime objects; consumers borrow through validated handles.
- Persist metadata and source/import information, never GPU allocations or runtime handles.
- See [Resource Identity and Lifecycle](RESOURCES.md) for the complete contract.
