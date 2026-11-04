# Dependency Policy

ZYNE is one Rust workspace. Dependencies are added only when an implemented feature needs them; planned future systems are not a reason to add dependencies early.

## Current direct dependencies

| Dependency | Crate | Purpose |
|---|---|---|
| thiserror 2 | zyne-core | Typed engine errors |
| serde 1 with derive | zyne-core | Configuration and resource identity serialization contracts |
| toml 0.8 | zyne-core | Parse and serialize project engine configuration |
| uuid 1 (v4, serde) | zyne-core | Persistent resource identity |
| tracing 0.1 | zyne-runtime | Instrumentation facade for engine diagnostics |
| tracing-subscriber 0.3 (fmt, env-filter) | zyne-runtime | Optional process-level log subscriber |

Workspace-wide versions are declared in the root Cargo.toml where shared by multiple crates.

## Review checklist for new dependencies

- Is the dependency needed by an implemented phase?
- Is its license compatible with the project license and distribution goals?
- Is it maintained and compatible with the supported stable Rust toolchain?
- Does it introduce unsafe code, platform constraints, or a large compile-time cost?
- Can the dependency remain behind the owning crate's public API?
- Is there a smaller standard-library or existing-dependency solution?

## Reviewed candidate, not yet added

| Dependency | Candidate version | Target crate | Decision | Planned phase |
|---|---:|---|---|---|
| winit | 0.30.13 | zyne-platform | Selected stable candidate; default features retain Linux X11 and Wayland support. Add only when window implementation begins. | 1.2 |

The winit decision and API constraints are recorded in [Phase 1.1 Review](development/PHASE-1-1-REVIEW.md). Do not add prerelease 0.31 to the initial implementation.

## Planned, not yet reviewed or added

wgpu, glam, bevy_ecs, rapier3d, kira, egui, gltf, image, and other subsystem dependencies are planned for later phases. They must not be treated as current dependencies until reviewed for their phase and added to a crate manifest.
