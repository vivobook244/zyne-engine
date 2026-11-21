# Phase 5.5.4 — Material Texture Integration

**Status:** IN PROGRESS — CPU TEXTURE DATA + GLTF IMAGE NORMALIZATION IMPLEMENTED; RUNTIME/GPU INTEGRATION REMAINS | **Branch:** `main` | **Prerequisite:** Phase 5.5.3 — Exposure Control

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
- [ ] Run formatting, strict Clippy, locked workspace tests/checks, and Ubuntu/Windows CI for the normalization implementation.

### 5.5.4.3 — Runtime Diffuse Texture Resolution

- [x] Resolve optional diffuse texture ResourceIds from loaded scene materials against registry metadata.
- [x] Validate that referenced resources are registered as `ResourceKind::Texture` and loaded in the typed CPU texture cache.
- [x] Return deterministic resolved/unresolved reports with entity, material, texture IDs, and typed issue reasons.
- [x] Preserve untextured materials as explicit fallback candidates rather than treating them as resolution failures.
- [ ] Run formatting, strict Clippy, locked workspace tests/checks, and Ubuntu/Windows CI for runtime resolution.

### Remaining integration work

1. Define a renderer-facing texture representation/handle that does not introduce a graphics dependency into `zyne-assets` or `zyne-scene`.
2. Resolve a material's optional diffuse texture ResourceId through the runtime asset registry/cache.
3. Upload and reuse GPU texture resources; avoid duplicate uploads for shared texture references.
4. Bind the resolved texture with the matching material draw. Materials without a diffuse texture retain a deterministic fallback texture.
5. Validate GPU-supported texture formats, dimensions, and upload-size constraints before GPU allocation.
6. Preserve scene-linear color handling so texture samples participate correctly in HDR lighting and final presentation.
7. Add unit/integration coverage for missing references, unloaded texture data, shared texture reuse, fallback behavior, and material-to-texture binding selection.
8. Extend `material_showcase` with distinct textured materials and keep its existing exposure controls and shadow receiver.

## Acceptance criteria

- [ ] A material with a valid diffuse texture ResourceId visibly samples that texture in the scene.
- [ ] A material without a diffuse texture uses the documented fallback and renders as before.
- [ ] Multiple materials can reference the same texture without duplicate GPU uploads.
- [ ] Missing or unresolved texture references produce a typed, actionable runtime error rather than silently binding an unrelated texture.
- [ ] Texture and material resources are released through explicit unload paths.
- [ ] HDR output, ACES tone mapping, exposure controls, lighting, and directional shadows remain functional.
- [ ] Automated tests cover resource resolution, reuse, fallback, and invalid input.
- [ ] Formatting, strict Clippy, locked workspace tests/checks, and Ubuntu/Windows CI pass.
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
