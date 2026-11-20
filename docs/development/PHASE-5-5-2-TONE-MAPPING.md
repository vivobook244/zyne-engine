# Phase 5.5.2 — Tone Mapping

**Status:** IMPLEMENTED — CI VALIDATION PENDING  
**Branch:** `main`  
**Prerequisite:** Phase 5.5.1 — HDR Foundation  
**Goal:** Convert scene-linear HDR color into a bounded SDR presentation signal while preserving the existing HDR scene target, forward renderer, lighting, depth, and shadow contracts.

## Architecture decisions

- Keep scene rendering in the existing `Rgba16Float` HDR target.
- Apply tone mapping only in the final fullscreen presentation pass.
- Use a fitted ACES approximation (Narkowicz curve) as the initial fixed tone-mapping operator.
- Clamp negative input values before evaluating the curve and clamp the output to the normalized display range.
- Keep the tone-mapped result in linear-light space; rely on an sRGB surface attachment for the display transfer when the selected surface format is sRGB.
- Do not add exposure controls in this milestone; those belong to Phase 5.5.3.
- Do not change material, lighting, shadow, or scene-rendering APIs.

## Implementation checklist

- [x] Add the fitted ACES tone-mapping function to the presentation shader.
- [x] Apply tone mapping after loading the HDR scene pixel and before writing to the surface.
- [x] Preserve the floating-point HDR scene target and existing depth/shadow passes.
- [x] Keep the presentation shader independent of filterable texture/sampler support by using `textureLoad`.
- [ ] Run formatting, strict Clippy, locked workspace tests/checks, and Windows/Ubuntu CI.
- [ ] Confirm the material and lighting showcase visually on a physical GPU.
- [ ] Review SDR output on the selected sRGB surface format and record visual observations.

## Color pipeline contract

The scene pass stores linear-light values in RGBA16Float. The presentation pass applies the fitted ACES curve to non-negative RGB values and writes the bounded result to the window surface. When the selected surface format is sRGB, the surface attachment performs the linear-to-sRGB display transfer. Tone mapping does not modify alpha semantics; presentation writes an opaque alpha value.

This is a fixed operator with no user-adjustable exposure. Exposure parameters and adaptation remain deferred to Phase 5.5.3.

## Deferred to later subphases

- Exposure parameters and exposure adjustment: Phase 5.5.3.
- Broader material, lighting, shadow, and presentation integration: Phase 5.5.4.
- Visual comparison, image-based regression checks, and cross-platform visual validation: Phase 5.5.5.
