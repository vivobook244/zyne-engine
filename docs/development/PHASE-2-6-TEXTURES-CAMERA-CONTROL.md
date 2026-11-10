# Phase 2.6 — Textures & Controllable Camera

**Status:** IN PROGRESS — CAMERA CONTROLS COMPLETE; CHECKER TEXTURE IMPLEMENTED, GPU VERIFICATION PENDING  
**Branch:** `main`  
**Prerequisite:** Phase 2.5 — Depth Buffer & 3D Foundation  
**Goal:** Complete the original Phase 2 acceptance target: render a textured 3D model and provide a camera that can be controlled through the engine's public runtime API.

## Scope and sequence

### 2.6.1 Camera contract and math

- Move camera/projection behavior out of the current one-off cube pipeline into reusable, testable camera data.
- Follow `docs/architecture/MATH.md`: right-handed world, +Y up, +Z engine forward, column-vector algebra, and [0, 1] projection depth.
- Validate viewport dimensions, field of view, near/far planes, and camera orientation.
- Keep GPU buffer layout and WGSL conversion inside `zyne-graphics`; do not leak wgpu types into core or math APIs.

### 2.6.2 Runtime input access

- Expose a stable, read-only per-frame input view to engine callbacks without exposing winit types.
- Preserve runtime ownership of event collection and input state.
- Support keyboard, mouse buttons, cursor movement, and scroll with documented frame semantics.
- Ensure focus loss and window suspension cannot leave camera controls stuck.

### 2.6.3 Camera controller

- Implement a small orbit/fly controller suitable for renderer validation.
- Provide deterministic movement/rotation speeds and delta-time handling.
- Avoid hard-wiring game-specific third-person behavior into the renderer; the controller is a sample/runtime utility, not gameplay policy.
- Add unit tests for pitch limits, zoom limits, zero delta, and focus-loss behavior where applicable.

### 2.6.4 Texture sampling

- [x] Add UVs to the mesh vertex format and a sampled 2D texture with a sampler.
- [x] Define explicit texture color-space handling (`Rgba8UnormSrgb`) and an embedded procedural checker texture for deterministic smoke testing.
- [x] Keep texture ownership and bind-group construction in `zyne-graphics`.
- [x] Defer external image loading, asset registries, material authoring, and hot reload to Phase 4/5.
- [ ] Verify checker texture sampling on a local desktop GPU.

### 2.6.5 Acceptance

- A textured 3D test model is visible on the supported desktop GPU path.
- Camera can be moved/rotated/zoomed using the public input contract.
- Resize updates camera aspect ratio without recreating unrelated GPU resources.
- Focus loss releases held controls; minimized/suspended windows do not advance camera movement.
- Tests cover camera math, input frame semantics, and texture/vertex layout contracts.
- `cargo fmt`, strict Clippy, workspace tests/check, and CI pass on Ubuntu and Windows.
- Local GPU smoke test confirms texture sampling and interactive camera behavior.

## Architecture guardrails

- Do not introduce ECS, scene serialization, asset registry, lighting, or physics in this phase.
- `zyne-core` remains independent of windowing and GPU APIs.
- `zyne-platform` continues to translate native input into portable events.
- `zyne-runtime` owns input state and exposes a read-only input contract to the engine.
- `zyne-graphics` owns camera GPU representation, texture resources, shaders, and draw pipeline.
- Do not make the renderer consume raw platform events directly.

## Definition of done

Phase 2 is complete only when the original roadmap acceptance criterion is met: a textured 3D model with a controllable camera, verified by CI and a local GPU smoke test.
