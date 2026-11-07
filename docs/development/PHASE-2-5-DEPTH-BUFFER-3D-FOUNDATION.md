# Phase 2.5 — Depth Buffer & 3D Foundation

**Status:** IMPLEMENTED — LOCAL VALIDATION REQUIRED  
**Branch:** `main`  
**Scope:** Replace the 2D triangle validation path with a perspective-projected 3D cube and a depth-tested render pass.

## Goals

- Add a Depth32Float depth attachment and clear it to 1.0 each frame.
- Enable depth writes and Less comparison in the 3D pipeline.
- Introduce 3D position attributes and a 36-vertex cube (12 triangles).
- Add model, view, and perspective projection matrices.
- Upload camera matrices through a uniform buffer and bind group.
- Recreate the depth texture when the drawable surface size changes.
- Preserve surface acquisition, resize, suspension, and presentation recovery.

## Architecture

`zyne-graphics` owns the depth texture, cube geometry, camera uniform, shader, and pipeline. The platform and runtime remain unchanged. Matrix helpers are intentionally private to the renderer for this first graphics milestone; a reusable public math API belongs to a later math phase.

## Acceptance checklist

- [ ] Cube uses 3D vertex positions.
- [ ] Perspective projection and view/model transforms are applied.
- [ ] Depth attachment is cleared and attached to the render pass.
- [ ] Depth testing and depth writes are enabled.
- [ ] Depth texture is recreated after surface resize.
- [ ] Existing resize/suspend/recovery behavior remains intact.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo clippy --locked --workspace --all-targets -- -D warnings` passes.
- [ ] `cargo test --locked --workspace` passes.
- [ ] `cargo check --locked --workspace` passes.
- [ ] CI passes on Ubuntu and Windows.
- [ ] Desktop smoke test visibly displays the 3D cube.

## Out of scope

Indexed mesh abstraction, reusable scene mesh assets, normals/lighting, materials, texture sampling, ECS integration, and interactive camera controls.
