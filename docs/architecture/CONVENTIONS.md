# Engine Conventions

This document records conventions that must remain consistent across engine crates. Where a convention is still provisional, it is explicitly marked.

## Coordinate system

- World coordinates are right-handed.
- +Y is world up.
- +Z is the intended world forward direction.
- One world unit is a logical engine unit; its physical scale is not fixed in Phase 0.
- Imported asset coordinate conversion must happen at an explicit importer boundary, not be scattered through gameplay or rendering code.

## Transform and math policy

The following implementation details are deliberately deferred until the math crate adopts concrete types and tests:

- Vector, quaternion, and matrix representation.
- Matrix storage layout and multiplication order.
- Local-to-parent and local-to-world composition.
- Camera view and projection conventions.
- GLTF node and animation coordinate conversion.

Do not infer a matrix convention from the +Z forward decision alone. These details must be selected together and locked by tests before renderer implementation.

## API and module policy

- Keep crate roots small; implementation belongs in focused modules.
- Expose stable crate APIs through deliberate `pub use` re-exports.
- Keep implementation modules private unless external consumers have a reason to access them.
- Public types and methods must have Rustdoc comments.
- Recoverable failures use `ZyneResult<T>` and typed `ZyneError` variants.
- Avoid panics for invalid project input or normal runtime failures.
- Do not expose third-party subsystem types through core contracts unless that coupling is intentional and documented.

## Logging policy

- Engine libraries emit structured events through `tracing`.
- Libraries must not initialize a process-wide subscriber.
- The executable/game host initializes diagnostics once, at startup.
- Default filter is `info`; `RUST_LOG` may override it.
- Use trace/debug for high-volume details, info for lifecycle milestones, warn for recoverable degraded behavior, and error for failed operations.

## Resource policy — provisional

Resource identity, typed handles, serialization stability, and load/unload lifecycle are not yet implemented. These decisions must be documented and tested before the asset pipeline phase.
