# Phase 3 — World, ECS & Scene

**Status:** IN PROGRESS — SCENE DOCUMENT CONTRACTS IMPLEMENTED  
**Branch:** `main`  
**Prerequisite:** Phase 2.6 — textured 3D rendering and controllable camera  
**Goal:** Introduce a small, serializable scene world that can contain multiple entities with stable identities and local transforms.

## Architecture decisions

### Ownership

- Add a dedicated `zyne-scene` crate. It owns the ECS world, scene-level components, scene document format, and scene validation.
- Use `bevy_ecs` as the ECS implementation. Keep its types behind the `zyne-scene` public API where practical.
- Keep `zyne-core` independent of ECS, gameplay, renderer, and editor dependencies.
- Keep `zyne-math` responsible for engine coordinate and transform conventions; do not introduce a second set of coordinate rules in the scene crate.
- Keep `zyne-graphics` independent of ECS. Rendering must consume extracted render-facing data rather than query a live ECS world.
- Runtime integration must not make platform or graphics code own gameplay state.

### Initial scene model

- A scene has a stable scene identity, a human-readable name, and a collection of entities.
- Each entity has a persistent scene-entity identity, a name, and a local 3D transform.
- Rotation uses quaternion component order `x, y, z, w`; identity is `(0, 0, 0, 1)`.
- World coordinates follow [Math and Coordinate Conventions](../architecture/MATH.md): right-handed, +Y up, +Z engine forward, column-vector algebra, and `T * R * S` composition.
- Scene entity IDs are persistent document identities. They are not ECS entity handles, resource IDs, or runtime slot handles.
- Loading a scene creates fresh ECS entities while preserving persistent scene-entity IDs.
- Initial persistence uses a versioned RON document. Runtime handles, pointers, GPU objects, and ECS entity coordinates must never be serialized.

Parent/child hierarchies, prefab instances, asset references, component reflection, editor metadata, and runtime hot reload are outside this first slice.

## Implementation sequence

### 3.1 Scene and transform contracts

- [x] Define validated `Transform3d` behavior and defaults.
- [x] Define persistent `SceneId` and `SceneEntityId` types without conflating them with `ResourceId`.
- [x] Define serializable scene document types and a schema version.
- [x] Reject duplicate entity IDs and non-finite or invalid transform values during construction/deserialization validation.

### 3.2 ECS world

- [ ] Add `bevy_ecs` to `zyne-scene`.
- [ ] Implement `Scene` ownership of an ECS `World`.
- [ ] Provide focused spawn, despawn, lookup, and read/write access APIs.
- [ ] Ensure every scene entity receives its persistent ID, name, and transform components.

### 3.3 Scene persistence

- [ ] Add RON encoding/decoding and validate documents before loading.
- [ ] Preserve scene and entity IDs across save/load.
- [ ] Add round-trip, empty-scene, duplicate-ID, invalid-transform, unsupported-version, and malformed-document tests.

### 3.4 Runtime/render vertical slice

- [ ] Expose scene functionality through the public runtime facade without leaking platform types.
- [ ] Define a renderer-neutral extraction boundary; do not let the renderer query ECS directly.
- [ ] Update the sample to create more than one transformed entity.
- [ ] Render the extracted entities using per-entity transforms while preserving the existing camera and texture behavior.
- [ ] Keep the headless smoke test independent of a physical GPU.

## Acceptance checklist

- [ ] A scene can contain multiple entities with distinct persistent IDs and transforms.
- [ ] Saving and loading preserves scene ID, entity IDs, names, and transforms.
- [ ] Invalid or duplicate persisted data fails explicitly; it never silently produces unrelated entities.
- [ ] The runtime/render path can consume multiple scene entities without depending on ECS internals.
- [ ] Existing orbit, pan, zoom, resize, texture, and depth behavior remains intact.
- [ ] Unit tests, formatting, strict Clippy, workspace checks, and Windows/Ubuntu CI pass.

## Dependency and lockfile policy

CI validates with `--locked`. New external dependencies must be accompanied by a Cargo-generated Cargo.lock update and verified on Windows and Linux. Do not hand-author transitive lock entries.

## Deferred to later phases

- Phase 4: GLTF/GLB and image import, persistent resource registry, typed runtime resource resolution, asset cache.
- Phase 5: material authoring, lighting, shadows, environment.
- Phase 10: editor-facing scene hierarchy and editing workflows.
