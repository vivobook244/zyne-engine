# Phase 4 — Asset & Resource Pipeline

**Status:** PHASE 4.1 COMPLETE — LOCAL VALIDATION AND WINDOWS/UBUNTU CI PASSED; EXPLICIT RUST 1.85 TOOLCHAIN CHECK REMAINS A FOLLOW-UP  
**Branch:** main  
**Prerequisite:** Phase 3 — World, ECS & Scene  
**Goal:** Import source assets into validated CPU-side engine data, then resolve persistent resource identities into safe runtime handles.

## Architecture decisions

- Introduce a dedicated zyne-assets crate for asset import and CPU-side asset representations.
- Keep zyne-core limited to stable ResourceId and typed Handle<T> contracts. Do not add filesystem, format, registry, or GPU dependencies to core.
- Keep importers independent from ECS and GPU APIs. Importers produce validated CPU data; zyne-graphics remains responsible for GPU allocation and upload.
- Persist ResourceId, resource kind, project-relative source path, and importer settings. Never persist runtime handles, slot indices, GPU objects, or pointers.
- A registry owns loaded runtime resources and validates handle index plus generation.
- Preserve the workspace MSRV at Rust 1.85. Pin dependencies where necessary; do not upgrade MSRV implicitly through a transitive dependency.
- Start with glTF 2.0 / GLB 2.0. Keep unsupported features explicit rather than silently dropping data.

## Implementation sequence

### 4.1 Asset import foundation

- [x] Add zyne-assets crate with CPU-side asset data types and glTF importer.
- [x] Import glTF 2.0 JSON and GLB 2.0 containers from local paths.
- [x] Extract triangle mesh primitives into engine-owned positions, optional normals/UV0, and normalized u32 indices.
- [x] Decode embedded and referenced images into engine-owned pixel buffers.
- [x] Preserve source mesh, primitive material, and image indices/names where available.
- [x] Validate required position data, index bounds, attribute lengths, finite values, triangle index counts, and image dimensions/byte lengths.
- [x] Return explicit errors for malformed documents, unsupported required extensions, and unsupported primitive topology.
- [x] Add fixture coverage and verify importer behavior on Windows/Linux CI.
- [x] Verify dependency resolution and Cargo.lock with `--locked` on the local environment and Windows/Linux CI.

### 4.2 Persistent resource registry

- [ ] Define resource metadata using ResourceId, resource kind, normalized project-relative source path, and importer settings.
- [ ] Register, find, rename, move, and remove metadata without changing identity on rename/move.
- [ ] Reject duplicate IDs and conflicting source ownership according to an explicit policy.
- [ ] Persist registry metadata independently of runtime objects.

### 4.3 Typed runtime resource resolution

- [ ] Implement registry-owned typed slots using Handle<T>.
- [ ] Validate both slot index and generation on lookup.
- [ ] Increment generation on release and reject stale handles.
- [ ] Keep handle construction and slot mutation inside the registry implementation.

### 4.4 Asset cache and lifecycle

- [ ] Implement explicit Unloaded, Loading, Loaded, and Failed states.
- [ ] Cache CPU-side imports and runtime resources without duplicate concurrent ownership.
- [ ] Define retry and unload behavior.
- [ ] Defer asynchronous loading, eviction policy, and dependency scheduling until measured need exists.

### 4.5 Scene resource references

- [ ] Add persistent ResourceId references to scene document contracts.
- [ ] Resolve references through the registry after scene load.
- [ ] Report missing or failed resources explicitly; never resolve to unrelated assets.
- [ ] Keep runtime handles out of RON scene documents.

## Phase 4.1 acceptance criteria

- A minimal valid GLB fixture imports at least one mesh primitive.
- A glTF fixture with external buffer/image references resolves paths relative to its source file.
- Position, normal, UV, and index data are extracted when present.
- Missing optional attributes are represented as absent, not fabricated.
- Invalid indices, non-finite values, malformed files, and unsupported required extensions fail explicitly.
- Image width, height, pixel format, and bytes are available without creating GPU resources.
- Unit tests, formatting, strict Clippy, locked workspace checks, and Windows/Ubuntu CI pass.
- No change to the Rust 1.85 MSRV; the workspace still declares `rust-version = "1.85"`.

**Validation note:** The current GitHub Actions workflow uses the stable Rust toolchain. Local and CI `--locked` checks passed, and dependencies are pinned according to the Rust 1.85 compatibility policy. A dedicated CI run using the exact Rust 1.85 toolchain is still recommended to verify the MSRV explicitly.

## Dependency policy

- Use the stable gltf 1.4.1 API with only the features required by the importer.
- The glTF import feature depends on the image 0.25 series. Pin image to 0.25.9 to keep the Rust 1.85 compatibility policy; do not allow image 0.25.10 (Rust 1.88) to enter the lockfile.
- Commit Cargo.lock updates generated by Cargo and verify with --locked in CI.

## Deferred

- Material authoring and PBR rendering: Phase 5.
- Scene hierarchy and prefab instantiation: later scene/editor phases.
- Animation and skinning runtime: later animation phase.
- GPU resource upload and device lifecycle: zyne-graphics.
- Hot reload and advanced cache eviction: only after the basic pipeline is stable.
