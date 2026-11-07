# Phase 2.3 — Frame Acquisition & Presentation

**Status:** IMPLEMENTATION IN PROGRESS — LOCAL VALIDATION REQUIRED  
**Branch:** `main`  
**Scope:** Acquire a window surface frame, clear its color attachment, submit GPU commands, and present the frame.

## 1. Goal

Turn the initialized and resize-aware GPU surface into a working presentation loop. The application must visibly clear the native window through the GPU on every active frame.

This phase deliberately does not introduce geometry, shader pipelines, materials, depth buffers, cameras, or scene rendering.

## 2. Ownership boundaries

| Layer | Responsibility |
|---|---|
| `zyne-platform` | Owns the native window and reports window/lifecycle events. |
| `zyne-graphics` | Acquires surface textures, handles surface errors, creates the clear render pass, submits command buffers, and presents frames. |
| `zyne-runtime` | Calls the graphics frame operation after engine updates and skips rendering while suspended. |
| `zyne-core` | Remains independent from window and graphics APIs. |

## 3. Frame flow

1. Skip rendering if the surface is suspended.
2. Acquire the current surface texture.
3. On `Lost` or `Outdated`, reconfigure the existing surface and retry acquisition once.
4. On `Timeout`, skip the current frame without terminating the application.
5. On `OutOfMemory`, return a fatal graphics error to the runtime.
6. Create a texture view and command encoder.
7. Begin a render pass that clears the color attachment to the engine background color.
8. Finish the command encoder and submit its command buffer to the GPU queue.
9. Present the acquired surface texture.

## 4. Explicitly out of scope

- Vertex/index buffers, meshes, and draw calls.
- Shader modules and render pipelines.
- Depth/stencil attachments.
- Cameras, transforms, materials, lighting, and scene extraction.
- Editor UI and frame capture tools.

## 5. Acceptance criteria

- [ ] Active frames acquire a surface texture and present it.
- [ ] The background clear is submitted through a render pass and GPU queue.
- [ ] Suspended surfaces do not attempt frame acquisition.
- [ ] `Lost` and `Outdated` surfaces are reconfigured and retried once.
- [ ] `Timeout` skips one frame without failing the application.
- [ ] `OutOfMemory` is propagated as a fatal graphics error.
- [ ] Runtime renders after successful engine update callbacks.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo clippy --locked --workspace --all-targets -- -D warnings` passes.
- [ ] `cargo test --locked --workspace` passes.
- [ ] `cargo check --locked --workspace` passes.
- [ ] Desktop smoke visibly presents a stable background on Windows and Linux where available.
- [ ] Resize and minimize/restore remain stable while frames are being presented.

## 6. Next phase

Phase 2.4 can introduce the first minimal render pipeline and triangle/mesh draw path, after the presentation lifecycle is verified.
