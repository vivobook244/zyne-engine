# Phase 2.4 — Render Pipeline & Shader Foundation

**Status:** IMPLEMENTATION IN PROGRESS — LOCAL VALIDATION REQUIRED  
**Branch:** `main`  
**Scope:** Establish the first shader-backed render pipeline and draw a basic triangle into the window surface.

## 1. Goal

Move beyond a clear-only frame and prove the renderer can consume vertex data, execute programmable vertex/fragment stages, and issue a draw call.

The expected result is a centered triangle with interpolated vertex colors over the existing dark blue-gray background.

## 2. Ownership boundaries

| Layer | Responsibility |
|---|---|
| `zyne-graphics` | Owns WGSL shader modules, pipeline layout, render pipeline, vertex buffer, and draw submission. |
| `zyne-runtime` | Continues to drive frame timing and calls the graphics renderer after engine updates. |
| `zyne-platform` | Continues to own the native window and event handling. |
| `zyne-core` | Remains independent from GPU and window APIs. |

## 3. Implementation

1. Add a small vertex format with 2D clip-space position and RGB color.
2. Create a static vertex buffer for three triangle vertices.
3. Add a WGSL vertex shader that maps vertex attributes to clip space.
4. Add a WGSL fragment shader that outputs interpolated vertex color.
5. Create a pipeline layout with no bind groups or push constants.
6. Create a render pipeline using the active surface format.
7. Bind the pipeline and vertex buffer in the frame render pass.
8. Issue a non-indexed triangle draw call.
9. Preserve surface acquisition, resize, suspension, and presentation behavior.

## 4. Explicitly out of scope

- Index buffers and reusable mesh abstraction.
- Depth/stencil buffer and depth testing.
- Camera matrices, world transforms, and 3D projection.
- Materials, textures, lighting, and scene extraction.
- Runtime shader hot reload and asset management.

## 5. Acceptance criteria

- [ ] Vertex data has a documented, tested memory layout.
- [ ] WGSL shader compiles on the selected GPU backend.
- [ ] Render pipeline uses the actual surface color format.
- [ ] Vertex buffer is bound and the triangle draw call is submitted.
- [ ] Triangle appears with interpolated vertex colors over the clear background.
- [ ] Existing frame acquisition and presentation recovery remains intact.
- [ ] Surface resize and suspend/resume remain functional.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo clippy --locked --workspace --all-targets -- -D warnings` passes.
- [ ] `cargo test --locked --workspace` passes.
- [ ] `cargo check --locked --workspace` passes.
- [ ] CI passes on Ubuntu and Windows.

## 6. Next phase

Phase 2.5 can introduce the depth attachment and a first 3D coordinate/depth-tested draw path. Mesh abstraction and indexed drawing should follow only after this minimal pipeline is stable.
