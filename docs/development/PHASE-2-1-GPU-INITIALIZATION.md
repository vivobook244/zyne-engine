# Phase 2.1 — Rendering Architecture & GPU Initialization

**Status:** IMPLEMENTATION IN PROGRESS — LOCAL VALIDATION REQUIRED  
**Branch:** `main`  
**Scope:** Establish the first GPU connection and configure a window-compatible presentation surface. No frame rendering or presentation is included.

## 1. Goal

Initialize a cross-platform GPU context for the existing desktop application. The first supported targets are Windows and Linux using wgpu's native backends.

## 2. Ownership boundaries

| Layer | Responsibility |
|---|---|
| `zyne-platform` | Owns the native window and exposes a cloneable, safe window-handle abstraction. It does not depend on wgpu. |
| `zyne-graphics` | Owns wgpu instance, compatible adapter, device, queue, surface, and initial surface configuration. |
| `zyne-runtime` | Initializes graphics after native window creation and retains the graphics context for the application lifetime. |
| `zyne-core` | Remains independent from OS and graphics APIs. |

The graphics crate may consume the platform window abstraction. The platform crate must never depend on graphics or runtime.

## 3. Dependency decisions

- `wgpu = 26.0.1`: selected because its documented MSRV (1.84) remains compatible with the workspace's declared Rust 1.85 minimum.
- `pollster = 0.4.0`: bridges wgpu's asynchronous adapter/device initialization into the current synchronous winit startup callback. Initialization occurs once, before engine startup, not per frame.
- `raw-window-handle = 0.6.2`: used by the platform window wrapper to provide safe native handles without exposing winit types.
- No shader, asset, material, ECS, physics, or rendering-pipeline dependencies are added.

## 4. Initialization sequence

1. Create the wgpu instance.
2. Create a surface from a cloned platform window handle.
3. Request a high-performance adapter compatible with that surface.
4. Request a device with no optional features and downlevel-compatible limits.
5. Query surface capabilities.
6. Prefer an sRGB format, FIFO presentation, and automatic alpha composition when available; otherwise select the first supported capability.
7. Configure the initial surface using the window's physical pixel dimensions.
8. Retain the context in runtime until application shutdown.

## 5. Error policy

- Surface, adapter, device, missing capability, and zero-size failures return typed graphics errors.
- Runtime converts graphics initialization failures to its existing subsystem error boundary.
- Engine `on_start` must not run if graphics initialization fails.
- No panic-based fallback or forced software adapter is used in this initial slice.

## 6. Explicitly out of scope

- Surface resize/reconfiguration and suspend/resume recovery.
- Acquiring, clearing, or presenting a surface frame.
- Render pipelines, shaders, buffers, textures, meshes, and draw calls.
- Depth buffer, camera, materials, lighting, and post-processing.

## 7. Acceptance criteria

- [ ] Platform window handle can be cloned and safely passed to wgpu without exposing winit types.
- [ ] Compatible adapter is selected for the created window surface.
- [ ] Device and queue are created with conservative required limits and no optional features.
- [ ] Surface capabilities are checked before configuration.
- [ ] Initial surface is configured with non-zero physical dimensions.
- [ ] Runtime initializes graphics before calling engine `on_start`.
- [ ] Graphics initialization errors are returned without panicking.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo clippy --locked --workspace --all-targets -- -D warnings` passes.
- [ ] `cargo test --locked --workspace` passes.
- [ ] `cargo check --locked --workspace` passes.
- [ ] Desktop smoke confirms the selected GPU and initial surface on Windows and Linux.

## 8. Follow-up

Phase 2.2 owns surface resize/reconfiguration, zero-sized/minimized window handling, and surface lifecycle recovery. Phase 2.3 integrates frame acquisition and presentation into the application loop.
