# Phase 5.5.4 — Material Texture Integration

**Status:** IMPLEMENTED; HEADLESS GPU FRAME INTEGRATION VALIDATED ON UBUNTU/WINDOWS; PHYSICAL GPU HDR/ACES/LIGHTING/SHADOW REGRESSION OPEN | **Branch:** `main` | **Prerequisite:** Phase 5.5.3 — Exposure Control

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

The graphics upload path, runtime texture cache, application synchronization path, per-draw binding, and textured showcase source are implemented. CI verifies compilation, formatting, Clippy, workspace tests/checks, headless lifecycle smoke tests, GPU texture upload/readback, and an offscreen production scene-pass render that reads back the selected material texture. CI GPU tests are automated and do not replace visual review on a physical GPU.

### Physical GPU showcase validation — 2026-11-24

The user ran `cargo run -p zyne-runtime --example material_showcase` on Linux. The runtime selected Intel UHD Graphics 610 (CFL GT1) through Vulkan (IntegratedGpu; Intel open-source Mesa driver). The supplied showcase screenshot and user confirmation establish:

- Blue ceramic diffuse texture is visible.
- Amber metal diffuse texture is visible.
- The checker fallback material remains visible.
- Resizing the application window changes the viewport with the window; the textures remain intact and visually unchanged.
- The process exits normally. Vulkan loader warnings about an optional DRM extension and unavailable Khronos validation layer did not prevent device initialization or rendering.

This proves physical-GPU texture sampling, fallback presentation, and resize stability. It does not prove shared GPU allocation reuse, explicit unload behavior, failed replacement preservation, or every HDR/ACES/lighting/shadow regression criterion. Exposure control was physically validated during Phase 5.5.3 and is not reopened by this resize test.

### Runtime cache lifecycle audit — 2026-11-24

The runtime cache lifecycle was tightened after reviewing revision-aware GPU synchronization:

- Mutable access through `AssetCache::get_mut` now changes the content revision, ensuring revision-aware GPU caches do not keep stale uploads after CPU-side edits.
- Revisions are treated as opaque equality tokens. Mutable-access revision changes wrap at `u64::MAX` rather than saturating and leaving a stale GPU cache indistinguishable from the current CPU value.
- `SceneMaterialRenderer::sync_materials` now reconciles material GPU allocations and material-to-texture mappings against material assignments in the active scene, and only prunes after all active materials synchronize successfully.
- `SceneMaterialRenderer::sync_textures` derives a deduplicated set of texture references from active scene material assignments. Shared textures remain resident until their final active material reference disappears.
- If material or texture resolution/upload fails, synchronization returns before cache reconciliation, preserving existing GPU allocations for a later retry.
- Regression tests cover material reassignment, entity despawn, texture reassignment, shared texture retention, orphan cleanup, revision changes, revision wraparound, and preservation of the old GPU resource when a replacement upload fails.

CI run `45355797688` passed on Ubuntu and Windows for commit `8f4e8c6`. The workspace test suite reports 235 passing tests on each platform. The new headless GPU test creates a real adapter/device, uploads both sRGB and Linear RGBA8 textures through the same production upload helper, creates the renderer-compatible bind group, copies texture pixels back to a mapped buffer, and verifies exact byte equality. This is automated GPU resource validation; it is not a visual rendering test and does not replace the physical-GPU showcase.

### GPU cache synchronization coverage — 2026-11-24

The production `SceneMaterialRenderer::sync_textures` path now delegates revisioned allocation, deduplication, and post-success pruning to one shared synchronization helper. The helper is exercised by focused runtime tests for:

- Duplicate references to one `ResourceId` invoke the upload closure only once.
- An unchanged revision reuses the existing cached resource without invoking upload again.
- A changed revision replaces the old resource only after the replacement closure succeeds.
- A shared resource remains resident while its ID remains in the active set and is pruned after the final reference disappears.
- A failed replacement preserves the old resource and skips stale-entry pruning.
- Explicit unload's shared removal primitive removes only the requested allocation.

CI run `45363725557` passed on Ubuntu and Windows for commit `c2bbabd`; formatting, strict Clippy, locked workspace tests/checks, and headless lifecycle smoke all passed. These tests exercise the exact generic synchronization helper used by the production GPU cache, using sentinel resources to observe upload calls and cache identity. They prove cache orchestration and failure semantics, but do not count driver-level GPU allocations or visually inspect a bound texture.


### Texture binding selection coverage — 2026-11-24

The renderer now routes batch texture selection through a small production helper used immediately before setting bind group 1. Focused tests verify both branches: a textured batch selects its material texture bind group, while an untextured batch selects the renderer-owned checker fallback. This tests binding selection logic but is not a full surface-backed frame-render integration test.

CI run `45364747525` (workflow run #688) passed on Ubuntu and Windows for commit `27e4d53`. Both jobs passed formatting, strict Clippy, workspace tests, workspace check, and the headless lifecycle smoke example. The run includes the two texture-binding selection tests and five texture-cache lifecycle tests. The earlier code commit `c2bbabd` also passed both CI platforms with the cache lifecycle tests.
## Remaining integration work

1. Add a surface-backed or otherwise end-to-end per-draw binding test if a reliable test boundary can be introduced. Fallback-versus-material bind-group selection is now covered by tests of the production selection helper (CI #688); cache orchestration is covered by production synchronization helper tests; actual GPU allocation and exact pixel readback are covered separately by the headless graphics test.
2. Complete a deliberate physical-GPU regression review of HDR output, ACES tone mapping, lighting, and directional shadows. Exposure control was already physically validated during Phase 5.5.3; do not repeat it without a specific regression reason.

## Acceptance criteria

- [x] A material with a valid diffuse texture ResourceId visibly samples that texture in the scene (physical GPU showcase, 2026-11-24).
- [x] A material without a diffuse texture uses the documented checker fallback in the physical GPU showcase (2026-11-24).
- [x] The production texture-cache synchronization helper deduplicates multiple active references to one texture ID and reuses an unchanged revision (CI run `45363725557`; sentinel-resource tests verify the upload closure is called once).
- [x] Missing or unresolved texture references produce a typed, actionable runtime error rather than silently binding an unrelated texture (`SceneRenderError::UnresolvedTexture` and `MissingCpuTexture`; resolution occurs before cache reconciliation).
- [x] Texture and material resources are released through explicit `SceneMaterialRenderer::unload_texture` and `unload_material` paths; scene reconciliation also prunes inactive GPU allocations after successful synchronization.
- [ ] HDR output, ACES tone mapping, lighting, and directional shadows pass a deliberate physical-GPU regression review. Exposure control was previously physically validated in Phase 5.5.3.
- [x] Automated tests cover resource resolution (registered/missing/wrong-kind/not-loaded), cache reuse and replacement, checker fallback selection, upload validation, and lifecycle reconciliation.
- [x] Formatting, strict Clippy, locked workspace tests/checks, and Ubuntu/Windows CI pass for the implementation commit (#619).
- [x] Dedicated headless GPU test exercises sRGB/Linear texture allocation, renderer-compatible bind-group creation, and exact pixel readback (CI run `45355797688`, Ubuntu and Windows).
- [x] Dedicated automated tests verify material texture versus checker-fallback bind-group selection through the production selection helper (CI run #688, Ubuntu and Windows).
- [x] A headless per-draw scene-pass integration test verifies the selected texture bind group is consumed by the production `encode_scene_passes` path and appears in GPU frame output via pixel readback (CI #700, Ubuntu and Windows). This is an offscreen scene-pass test, not a surface-backed presentation test. Cache reuse, revision replacement, shared-reference lifetime, pruning, failed-replacement safety, and the unload removal primitive are covered by production-helper tests (CI run `45363725557`); device-level allocation counts remain unverified.
- [x] Physical-GPU showcase confirms blue/amber texture sampling, checker fallback, and resize stability (2026-11-24).
- [ ] Physical-GPU regression review confirms HDR, ACES, lighting, and directional shadows remain correct.

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


### Acceptance checklist reconciliation — 2026-11-26

A source audit of the current `main` branch reconciled three stale unchecked acceptance boxes with code and tests already present in the repository:

- `SceneMaterialRenderer::sync_textures` validates registry membership, resource kind, CPU cache revision, and CPU payload before GPU cache synchronization. It returns typed `SceneRenderError::UnresolvedTexture` or `MissingCpuTexture` errors; it does not silently substitute the checker texture for an explicit broken diffuse reference.
- `SceneMaterialRenderer::unload_texture` and `unload_material` provide explicit GPU-resource removal. Normal scene synchronization also prunes inactive material/texture allocations only after successful resolution/upload.
- `scene_materials.rs` tests cover resolved and missing texture references; `scene_renderer.rs` tests cover shared-resource lifetime, reassignment, stale-resource pruning, and failed replacement preservation; graphics tests cover invalid upload dimensions/payloads and checker-versus-material binding selection.

These criteria were implemented before this audit; this entry records the verification and corrects checklist bookkeeping, not a new runtime implementation. Remaining acceptance work is still the end-to-end encoded-frame binding test and the deliberate physical-GPU HDR/ACES/lighting/directional-shadow regression review. Do not mark Phase 5.5.4 complete until those are addressed or explicitly accepted as deferred.


### Headless production scene-pass render validation — 2026-11-26

A focused test, `context::tests::headless_scene_pass_renders_the_selected_material_texture`, creates a headless WGPU adapter/device, constructs the production `CubePipeline`, uploads a solid red linear RGBA8 texture, creates a material and mesh batch using that texture, and calls the production `encode_scene_passes` helper. It copies the offscreen render target into a mapped readback buffer and asserts that the center pixel is strongly red with low green/blue channels. The test passed on both Ubuntu and Windows in CI workflow run #700 (commit `c9d60cc`). The companion GPU texture upload/readback test also passed on both platforms. This closes the previous unchecked acceptance item for selected texture consumption by an encoded/rendered scene pass. It does not validate the window surface presentation pass or replace physical-GPU visual regression review for HDR, ACES, lighting, and directional shadows.


### HDR presentation GPU readback validation — 2026-11-27

The production `PresentationPipeline` now has a headless GPU readback regression test, `presentation_gpu_readback_matches_srgb_display_output_for_both_surface_formats`. It renders a known linear 0.18 gray HDR input through the production presentation pipeline into both `Rgba8Unorm` and `Rgba8UnormSrgb` targets, then reads the output pixel back from the GPU. The test checks neutral RGB channels, opaque alpha, the expected approximately 141 sRGB baseline at 0 EV, and monotonically increasing brightness across -1, 0, and +1 EV.

CI workflow run #717 passed on Ubuntu and Windows for commit `b0c2eb851e0cd43d15ceab1f952e908c62634e77`. Both platforms passed formatting, strict Clippy, workspace tests, workspace check, and the headless lifecycle smoke example. This verifies the production HDR-to-display presentation pipeline offscreen for both sRGB and non-sRGB target formats. It does not replace a window surface-backed test or the outstanding physical-GPU visual review of HDR output, ACES tone mapping, lighting, and directional shadows.
