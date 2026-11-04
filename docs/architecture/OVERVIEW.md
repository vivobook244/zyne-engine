# Architecture Overview

## Structure

ZYNE is a modular monolith implemented as one Rust Cargo workspace. Crates are separated by responsibility and communicate through explicit public APIs and data contracts.

## Initial dependency direction

~~~text
zyne-core       zyne-math
     ^               ^
     |               |
zyne-platform   zyne-graphics
                     ^
                     |
                zyne-runtime
~~~

The initial graph is intentionally small. Add crates when their phase requires them; avoid empty placeholder crates.

## Boundary rules

- Core must not depend on renderer, editor, or gameplay.
- Math provides shared mathematical conventions and types.
- Platform owns OS integration, windows, input devices, and application lifecycle.
- Graphics consumes render-ready data and does not know gameplay rules.
- Runtime is the public facade for game projects.
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

Target a right-handed world coordinate system with +Y up. The forward-axis convention (+Z forward) must remain consistent across camera controls, GLTF import, and GPU projection code. Conversion belongs at explicit boundaries.

## Quality strategy

Every phase should provide a runnable demo and appropriate tests. Prefer a vertical slice from window to mesh to camera to scene to playable character over broad but unintegrated scaffolding.
