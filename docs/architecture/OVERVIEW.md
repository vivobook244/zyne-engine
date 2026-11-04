# Architecture Overview

## Structure

ZYNE is a modular monolith implemented as one Rust Cargo workspace. Crates are separated by responsibility and communicate through explicit public APIs and data contracts.

## Current workspace dependency graph

An arrow `A -> B` means crate A directly depends on crate B.

~~~text
zyne-runtime -> zyne-core
             -> zyne-platform -> zyne-core
             -> zyne-graphics -> zyne-core
                              -> zyne-math

zyne-math has no workspace-crate dependencies.
zyne-core has no workspace-crate dependencies.
~~~

This is the graph currently declared by the crate manifests. Cargo metadata and workspace compilation validate that it resolves without a dependency cycle. The graph may grow when later phases introduce subsystem dependencies; preserve the direction rules below.

## Crate ownership

| Crate | Owns | Must not own |
|---|---|---|
| zyne-core | Engine-wide contracts, configuration, errors, version, resource identity/handles | OS, GPU, gameplay, editor |
| zyne-math | Shared math types and coordinate conventions | Windowing, rendering policy, gameplay |
| zyne-platform | OS integration, native windows, OS event acquisition/translation, platform input acquisition | GPU rendering, engine lifecycle policy, and game rules |
| zyne-graphics | GPU resources, render pipeline, materials, render-facing data | Gameplay policy and editor workflows |
| zyne-runtime | Public host facade, diagnostics setup, engine lifecycle policy, update scheduling, frame-time policy, subsystem orchestration | Native OS implementation and subsystem implementation details that belong in their owning crates |

## Boundary rules

- Core must not depend on renderer, editor, platform, or gameplay.
- Math provides shared mathematical conventions and types and remains independent of other workspace crates.
- Platform may depend on core contracts, but not graphics or gameplay.
- Graphics may depend on core and math, but not platform or gameplay.
- Runtime composes core, platform, and graphics; it owns engine lifecycle policy and update/frame scheduling, but delegates native event-loop mechanics and OS event translation to platform.
- Platform owns backend integration (planned: winit); runtime decides what platform events mean for engine lifecycle. Core remains independent of windows and OS event loops.
- See [Platform Architecture](PLATFORM.md) for event-loop ownership, public API/backend separation, error boundaries, timing ownership, and deferred implementation decisions.
- Physics does not directly control animation.
- Asset loading does not depend on editor UI.
- Scene/world data does not depend on editor implementation.
- Editor consumes public engine APIs.
- Avoid circular crate dependencies.

## Engine contracts and configuration

The `zyne-core` crate owns engine-wide contracts: `Engine` lifecycle callbacks, validated `EngineConfig`, immutable `EngineContext`, `EngineVersion`, and `ZyneError`/`ZyneResult`. Project configuration is TOML; subsystem-specific settings stay with their owning crate. See [configuration](../CONFIGURATION.md) and [engine conventions](CONVENTIONS.md).

The `zyne-runtime` crate exposes `init_logging()` as the host-level tracing subscriber setup. Library crates must not initialize a global subscriber.

## Planned building blocks

| Concern | Foundation | ZYNE responsibility |
|---|---|---|
| Window/input | winit | Lifecycle and input API |
| GPU rendering | wgpu + WGSL | Render passes, materials, resource ownership |
| Math | glam | Engine conventions and public facade |
| ECS | bevy_ecs | Components, systems, scene integration |
| Physics | rapier3d | Physics lifecycle and gameplay API |
| Animation | gltf + custom runtime | Skeleton, clips, blending, state machine |
| Audio | kira | Audio scene, emitters, listener, playback |
| Editor UI | egui | Studio workflows and integration |
| Assets | gltf, image, serde | Import, handles, registry, cache |

Dependencies are introduced and validated when needed, not pulled into the foundation without use.

## Rendering direction

Initial renderer: forward rendering with Blinn–Phong lighting. Intended pass sequence: scene extraction, frustum culling, shadow pass, opaque pass, transparent pass, post-processing, presentation. Cascaded shadows and advanced effects are deferred.

## Coordinate conventions

The authoritative contract is [Math and Coordinate Conventions](MATH.md): right-handed world, +Y up, +Z engine forward, column-vector algebra, and a [0, 1] projection depth range. Import conversion belongs at explicit asset boundaries.

## Quality strategy

Every phase should provide a runnable demo and appropriate tests. Prefer a vertical slice from window to mesh to camera to scene to playable character over broad but unintegrated scaffolding.
