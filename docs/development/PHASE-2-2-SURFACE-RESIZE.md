# Phase 2.2 — Surface Resize & Reconfiguration

**Status:** DONE — IMPLEMENTED, LOCALLY VALIDATED, AND CI VERIFIED  
**Branch:** `main`  
**Scope:** Keep the GPU presentation surface synchronized with native window dimensions and lifecycle events. Frame acquisition and presentation are handled in Phase 2.3.

## 1. Goal

Handle physical-pixel size changes safely after initial GPU initialization. A minimized or otherwise zero-sized window must never be passed to `wgpu::Surface::configure`, and a restored window must be able to reconfigure the existing surface.

## 2. Ownership boundaries

| Layer | Responsibility |
|---|---|
| `zyne-platform` | Reports resize, DPI scale-factor, suspend, and resume events; exposes the current physical client-area size. |
| `zyne-graphics` | Owns surface configuration, detects redundant resize requests, suspends zero-sized surfaces, and reconfigures valid dimensions. |
| `zyne-runtime` | Forwards native window and lifecycle changes to the graphics context before queuing events for normal runtime processing. |
| `zyne-core` | Remains independent from window and graphics APIs. |

## 3. Behavior

1. A physical `Resized` event updates the surface using the supplied physical pixel dimensions.
2. A zero width or height marks the surface suspended and skips configuration. The previous valid dimensions remain stored.
3. A subsequent non-zero size reconfigures the surface and clears the suspended state.
4. A repeated non-zero size is ignored when the surface is already active and dimensions are unchanged.
5. A DPI scale-factor change reads the current physical client-area size from `PlatformWindow` and synchronizes the surface.
6. A platform suspend marks the surface unavailable. Resume reads the current physical size and reconfigures, even if the dimensions match the previous configuration.
7. No new GPU device, queue, adapter, or surface is created during resize.

## 4. Explicitly out of scope

- Acquiring or presenting a surface texture.
- Handling `SurfaceError::Lost` or `Outdated` during frame acquisition; that belongs with frame lifecycle in Phase 2.3.
- Render pipelines, shaders, buffers, textures, meshes, and draw calls.
- Depth buffer, camera, materials, lighting, and post-processing.

## 5. Acceptance criteria

- [x] Resize events update surface dimensions in physical pixels.
- [x] Zero-sized dimensions never reach `surface.configure`.
- [x] Restoring a non-zero size reconfigures the surface.
- [x] Suspend/resume reconfigures the surface safely.
- [x] DPI changes synchronize using the current physical window size.
- [x] Unchanged dimensions do not trigger redundant configuration.
- [x] No adapter/device/surface recreation occurs during resize.
- [x] `cargo fmt --all -- --check` passes.
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings` passes.
- [x] `cargo test --locked --workspace` passes.
- [x] `cargo check --locked --workspace` passes.
- [x] Desktop smoke confirms resize, minimize/restore, and DPI behavior on Windows and Linux where available.

## 6. Follow-up

Phase 2.3 adds frame acquisition, command submission, surface presentation, and recovery for lost or outdated surfaces.
