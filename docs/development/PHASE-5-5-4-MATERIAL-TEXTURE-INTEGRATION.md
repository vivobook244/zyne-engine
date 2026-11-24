# Phase 5.5.4 — Material Texture Integration

**Status:** GPU TEXTURE UPLOAD + RUNTIME BINDING IMPLEMENTED; SHOWCASE / PHYSICAL GPU VALIDATION REMAINS | **Branch:** `main` | **Prerequisite:** Phase 5.5.3 — Exposure Control

## Audit findings

The current renderer already integrates scene materials, directional/point/spot lighting, one directional shadow map, HDR scene rendering, ACES presentation, and fixed EV exposure. The remaining concrete material gap is texture binding:

- `BlinnPhongMaterial` stores an optional persistent `diffuse_texture: Option<ResourceId>`.
- Scene material resolution currently converts material parameters into `MaterialData` containing ambient, diffuse, specular, and shininess only.
- The graphics pipeline binds one renderer-owned checker texture at bind group 1 for every material draw.
- The fragment shader samples that checker texture and multiplies it by vertex color and material diffuse color.
- Consequently, a material's persistent diffuse texture reference is not yet resolved to a GPU texture or sampled by its material draw.

This phase addresses that missing connection. It does not replace the current Blinn–Phong model or introduce a PBR workflow.

## Implementation sequence

### 5.5.4.1 — CPU Texture Data & Validation

- [x] Add renderer-independent, owned RGBA8 `TextureData` in `zyne-assets`.
- [x] Record sRGB versus linear RGB interpretation without introducing GPU types.
- [x] Validate non-zero dimensions, checked byte-length arithmetic, and exact tightly packed RGBA8 payload length.
- [x] Add a typed texture cache alias using the existing generic asset cache.
- [x] Add unit tests for valid data, color-space metadata, zero dimensions, incorrect byte lengths, and arithmetic overflow.
- [x] Run formatting, strict Clippy, locked workspace tests/checks, and Ubuntu/Windows CI for the normalization implementation.

### 5.5.4.2 — glTF Image Normalization

- [x] Convert imported 8-bit R, RG, RGB, and RGBA images into validated tightly packed RGBA8 CPU data.
- [x] Expand grayscale channels deterministically and synthesize opaque alpha when the source has no alpha channel.
- [x] Preserve source channel values and require the caller to select the intended color-space interpretation.
- [x] Reject 16-bit and floating-point source formats explicitly until a deliberate conversion policy is designed.
- [x] Run formatting, strict Clippy, locked workspace tests/checks, and Ubuntu/Windows CI for the normalization implementation.

### 5.5.4.3 — Runtime Diffuse Texture Resolution

- [x] Resolve optional diffuse texture ResourceIds from loaded scene materials against registry metadata.
- [x] Validate that referenced resources are registered as `ResourceKind::Texture` and loaded in the typed CPU texture cache.
- [x] Return deterministic resolved/unresolved reports with entity, material, texture IDs, and typed issue reasons.
- [x] Preserve untextured materials as explicit fallback candidates rather than treating them as resolution failures.
- [x] Run formatting, strict Clippy, locked workspace tests/checks, and Ubuntu/Windows CI for runtime resolution (CI #603, commit `5e90a682`).

### 5.5.4.4 — GPU Texture Upload, Cache & Draw Binding

- [x] Add graphics-owned `GpuTexture` with texture, view, sampler, bind group, dimensions, and storage format.
- [x] Add `GraphicsContext::create_texture` for validated tightly packed RGBA8 uploads.
- [x] Enforce non-zero dimensions, the device's maximum 2D texture dimension, checked byte-length arithmetic, and exact payload size before allocation.
- [x] Map CPU sRGB/linear metadata to `Rgba8UnormSrgb` / `Rgba8Unorm` so sampled color enters scene lighting with the intended interpretation.
- [x] Extend `MeshRenderBatch` with optional per-batch texture binding while retaining the checker fallback.
- [x] Add a `ResourceId`-keyed GPU texture cache to `SceneMaterialRenderer`; repeated material references reuse one upload.
- [x] Add explicit texture unload and typed unresolved/missing CPU/GPU texture errors.
- [x] Include texture identity in scene draw grouping so batches with different diffuse textures cannot share the wrong bind group.
- [x] Add the `SceneTextureEngine` contract and `Application::run_scene_with_textures` synchronization path.
- [x] Extend `material_showcase` with blue ceramic and amber metal diffuse patterns; retain the matte checker-fallback material, exposure controls, and shadow receiver.
- [x] Run formatting, strict Clippy, locked workspace tests/checks, and headless lifecycle smoke tests on Ubuntu and Windows (CI #670 passed on Ubuntu and Windows; 234 tests passed per platform).

### GPU integration audit — 2026-11-21

The current `main` branch was inspected before changing the renderer. The following existing contracts are confirmed:

- `GraphicsContext` owns the `wgpu::Device` and `wgpu::Queue` and is the only public GPU creation boundary.
- `CubePipeline` currently owns the checker texture and its texture/sampler bind-group layout. The scene shader consumes that resource at bind group 1.
- Bind group 3 is reserved for per-material Blinn–Phong uniform data; bind group 2 is lighting and bind group 0 is camera/shadow state.
- `MeshRenderBatch` currently carries mesh, transforms, and optional material. Draws are grouped by mesh/material before command encoding.
- `SceneMaterialRenderer` already caches GPU materials by persistent `ResourceId` and exposes explicit unload behavior. This is the established pattern to mirror for textures.
- `TextureData` owns tightly packed RGBA8 CPU bytes and records `Srgb` or `Linear` interpretation. The graphics layer must choose `Rgba8UnormSrgb` or `Rgba8Unorm` accordingly.
- The HDR scene render target and lighting shader remain downstream of texture sampling. Texture sampling must stay in the scene pass and must not alter the HDR presentation pass.

### Integration decisions

1. Add a graphics-owned `GpuTexture` and a `GraphicsContext::create_texture` upload boundary. It will own the GPU texture, view, sampler, and bind group; no GPU type enters assets, scene, or serialized documents.
2. Reuse the existing checker texture layout at bind group 1. Keep the checker bind group as the explicit fallback for materials without a diffuse texture.
3. Extend render batches to carry an optional `GpuTexture`. Bind the selected texture for each material batch; retain existing material uniform binding at group 3.
4. Add a texture cache to `SceneMaterialRenderer` keyed by texture `ResourceId`. Repeated references reuse one upload; explicit unload removes the owned GPU texture.
5. Runtime synchronization must resolve all texture references before issuing uploads. Missing registry entries, wrong resource kinds, and unloaded CPU payloads remain typed errors; do not silently replace a broken explicit reference with the checker fallback.
6. Validate dimensions against `device.limits().max_texture_dimension_2d` and validate checked upload layout before creating GPU objects. Use the texture's recorded color space to select the GPU format.
7. Extend draw grouping to include texture identity (or equivalent batch identity) so different diffuse textures cannot accidentally share one draw binding.

### Implementation boundary

The graphics upload path, runtime texture cache, application synchronization path, per-draw binding, and textured showcase source are implemented. CI verifies compilation, formatting, Clippy, workspace tests/checks, and headless lifecycle smoke tests; it does not instantiate a physical GPU or visually inspect the showcase. Dedicated GPU upload/binding coverage and physical-GPU showcase validation remain pending.

### Runtime cache lifecycle audit — 2026-11-24

The runtime cache lifecycle was tightened after reviewing revision-aware GPU synchronization:

- Mutable access through `AssetCache::get_mut` now changes the content revision, ensuring revision-aware GPU caches do not keep stale uploads after CPU-side edits.
- Revisions are treated as opaque equality tokens. Mutable-access revision changes wrap at `u64::MAX` rather than saturating and leaving a stale GPU cache indistinguishable from the current CPU value.
- `SceneMaterialRenderer::sync_materials` now reconciles material GPU allocations and material-to-texture mappings against material assignments in the active scene, and only prunes after all active materials synchronize successfully.
- `SceneMaterialRenderer::sync_textures` derives a deduplicated set of texture references from active scene material assignments. Shared textures remain resident until their final active material reference disappears.
- If material or texture resolution/upload fails, synchronization returns before cache reconciliation, preserving existing GPU allocations for a later retry.
- Regression tests cover material reassignment, entity despawn, texture reassignment, shared texture retention, orphan cleanup, revision changes, revision wraparound, and preservation of the old GPU resource when a replacement upload fails.

CI #670 passed on Ubuntu and Windows for commit `30ef5ed`. The workspace test suite reports 234 passing tests on each platform. These tests validate scene-reference reconciliation and cache contracts; they do not instantiate or visually inspect a physical GPU texture.

## Remaining integration work

1. Add focused automated coverage for GPU texture upload validation, cache reuse/unload, fallback selection, and per-batch binding. Existing CPU-side resolution tests and cross-platform CI pass, but they do not exercise an actual GPU texture allocation.
2. Run `material_showcase` on a physical GPU and confirm the two diffuse textures are visibly sampled while HDR, ACES, exposure, lighting, and directional shadows remain visually correct.

## Acceptance criteria

- [ ] A material with a valid diffuse texture ResourceId visibly samples that texture in the scene.
- [ ] A material without a diffuse texture uses the documented fallback and renders as before.
- [ ] Multiple materials can reference the same texture without duplicate GPU uploads.
- [ ] Missing or unresolved texture references produce a typed, actionable runtime error rather than silently binding an unrelated texture.
- [ ] Texture and material resources are released through explicit unload paths.
- [ ] HDR output, ACES tone mapping, exposure controls, lighting, and directional shadows remain functional.
- [ ] Automated tests cover resource resolution, reuse, fallback, and invalid input.
- [x] Formatting, strict Clippy, locked workspace tests/checks, and Ubuntu/Windows CI pass for the implementation commit (#619).
- [ ] Dedicated automated tests exercise GPU texture allocation, cache reuse, fallback, and per-draw binding.
- [ ] Physical-GPU showcase validation confirms textured materials and regression checks.

## Explicit non-goals

- PBR material model or metallic/roughness workflow.
- Normal, roughness, metallic, emissive, or occlusion maps.
- Alpha blending, transparent sorting, and refraction.
- Multiple shadow maps, point/spot shadows, or cascaded shadows.
- Automatic exposure, bloom, color grading, or a general post-processing graph.
- Editor material UI.

## Implementation guidance

Keep ownership boundaries intact: assets own validated CPU texture data and persistent ResourceIds; runtime resolves scene/material references and coordinates upload lifetime; graphics owns GPU texture objects, bind groups, and sampling. Do not store GPU handles in scene documents or asset data.

Keep the existing checker texture as an explicit fallback for untextured materials during this phase. A later phase may replace the fallback policy after texture sampling is established and visually validated.

## Validation notes

The existing renderer writes scene-linear values to an RGBA16Float target, then applies exposure and ACES in the presentation pass. Diffuse texture sampling must happen in the scene shader before lighting and must not bypass that HDR-to-SDR pipeline.

The current renderer uses one directional shadow map. Texture integration must not change shadow ownership, scene creation-order behavior, or shadow pass geometry submission.
