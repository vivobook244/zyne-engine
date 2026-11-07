# Phase 2.5 — Depth Buffer & 3D Foundation

**Status:** IMPLEMENTED — CI PASSED; LOCAL GPU SMOKE TEST PASSED  
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

- [x] Cube uses 3D vertex positions.
- [x] Perspective projection and view/model transforms are applied.
- [x] Depth attachment is cleared and attached to the render pass.
- [x] Depth testing and depth writes are enabled.
- [x] Depth texture is recreated after surface resize.
- [x] Existing resize/suspend/recovery behavior remains intact.
- [x] `cargo fmt --all -- --check` passes (CI).
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings` passes (CI).
- [x] `cargo test --locked --workspace` passes (CI).
- [x] `cargo check --locked --workspace` passes (CI).
- [x] CI passes on Ubuntu and Windows.
- [x] Desktop smoke test visibly displays the 3D cube (Intel UHD 610 / Vulkan, Linux).

## Visual validation notes

The first visible cube revealed that the initial model rotation nearly cancelled the camera azimuth, making the cube appear almost front-on. The camera now uses a diagonal `[4.0, 2.8, 4.0]` viewpoint with a modest `0.15` radian model rotation to expose distinct front, side, and top faces.

## Out of scope

Indexed mesh abstraction, reusable scene mesh assets, normals/lighting, materials, texture sampling, ECS integration, and interactive camera controls.
