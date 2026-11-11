# Phase 3 — World, ECS & Scene

**Status:** IN PROGRESS — PHASE 3.4 IMPLEMENTED; LOCAL GPU VALIDATION PENDING  
**Branch:** \`main\`  
**Prerequisite:** Phase 2.6 — textured 3D rendering and controllable camera  
**Goal:** Introduce a small, serializable scene world that can contain multiple entities with stable identities and local transforms.

## Architecture decisions

### Ownership

- Add a dedicated \`zyne-scene\` crate. It owns the ECS world, scene-level components, scene document format, and scene validation.
- Use \`bevy_ecs\` as the ECS implementation. Keep its types behind the \`zyne-scene\` public API where practical.
- Keep \`zyne-core\` independent of ECS, gameplay, renderer, and editor dependencies.
- Keep \`zyne-math\` responsible for engine coordinate and transform conventions.
- Keep \`zyne-graphics\` independent of ECS. Rendering must consume extracted render-facing data rather than query a live ECS world.
- Runtime integration must not make platform or graphics code own gameplay state.
- Scene entity IDs are persistent document identities, not ECS entity handles or runtime slot handles.
- Initial persistence uses a versioned RON document. Runtime handles, pointers, GPU objects, and ECS entity coordinates must never be serialized.

Parent/child hierarchies, prefab instances, asset references, component reflection, editor metadata, and runtime hot reload are outside this first slice.

## Implementation sequence

### 3.1 Scene and transform contracts

- [x] Define validated Transform3d behavior and defaults.
- [x] Define persistent SceneId and SceneEntityId types.
- [x] Define serializable scene document types and a schema version.
- [x] Reject duplicate entity IDs and invalid transform values.

### 3.2 ECS world

- [x] Add bevy_ecs to zyne-scene.
- [x] Implement Scene ownership of an ECS World.
- [x] Provide focused spawn, despawn, lookup, and read/write access APIs.
- [x] Ensure every scene entity receives persistent ID, name, and transform components.

### 3.3 Scene persistence

- [x] Add RON encoding/decoding and validate documents before loading.
- [x] Preserve scene and entity IDs across save/load.
- [x] Add round-trip, empty-scene, duplicate-ID, invalid-transform, unsupported-version, and malformed-document tests.

### 3.4 Runtime/render vertical slice

- [x] Expose scene functionality through the public runtime facade without leaking platform types.
- [x] Define a renderer-neutral extraction boundary; do not let the renderer query ECS directly.
- [x] Update the sample to create more than one transformed entity.
- [x] Render extracted entities using per-entity transforms while preserving camera and texture behavior.
- [x] Keep the headless smoke test independent of a physical GPU.

## Acceptance checklist

- [x] A scene can contain multiple entities with distinct persistent IDs and transforms.
- [x] Saving and loading preserves scene ID, entity IDs, names, and transforms.
- [x] Invalid or duplicate persisted data fails explicitly.
- [x] Runtime/render path consumes multiple scene entities without depending on ECS internals.
- [ ] Existing orbit, pan, zoom, resize, texture, and depth behavior remains intact.
- [x] Unit tests, formatting, strict Clippy, workspace checks, and Windows/Ubuntu CI pass.
- [ ] Local GPU validation confirms multi-entity rendering, camera controls, texture sampling, resize, and depth behavior.

## Dependency and lockfile policy

CI validates with --locked. New external dependencies require a Cargo-generated Cargo.lock update verified on Windows and Linux. Do not hand-author transitive lock entries.

## Deferred to later phases

- Phase 4: GLTF/GLB and image import, persistent resource registry, typed runtime resource resolution, asset cache.
- Phase 5: material authoring, lighting, shadows, environment.
- Phase 10: editor-facing scene hierarchy and editing workflows.
