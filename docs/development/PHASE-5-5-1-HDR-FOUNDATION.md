# Phase 5.5.1 — HDR Foundation

**Status:** IMPLEMENTED — CI VALIDATED; PHYSICAL GPU VISUAL VALIDATION PENDING  
**Branch:** `main`  
**Prerequisite:** Phase 5.4.5 — Lighting Documentation  
**Goal:** Preserve scene-linear lighting output in a floating-point render target before display presentation, without changing the forward-rendering or Blinn–Phong contracts.

## Architecture decisions

- Keep the existing forward renderer and Blinn–Phong lighting model.
- Render the scene color pass into `Rgba16Float`, with render-attachment and texture-binding usage.
- Validate that the selected adapter supports both required usages before creating the graphics device resources.
- Keep depth and directional shadow passes independent from the HDR color target.
- Present the HDR target through a dedicated fullscreen-triangle pass.
- Use `textureLoad` in the initial presentation pass so the HDR texture does not require a filterable-float capability or sampler.
- Keep the presentation pass as a direct linear-value transfer. Do not introduce tone mapping or exposure in this milestone.
- Recreate the HDR target and its presentation bind group when the window surface is resized or restored from suspension.

## Implementation checklist

- [x] Add an RGBA16-float HDR scene target.
- [x] Validate adapter format usages before target creation.
- [x] Route the Blinn–Phong scene pipeline into the HDR target.
- [x] Add a dedicated fullscreen presentation pipeline.
- [x] Keep the directional shadow pass and depth buffer behavior intact.
- [x] Recreate HDR resources on non-zero surface resize.
- [x] Add a WGSL parser/validator test for the presentation shader.
- [x] Run formatting, strict Clippy, locked workspace tests/checks, and Windows/Ubuntu CI (GitHub Actions run [#547](https://github.com/vivobook244/zyne-engine/actions/runs/44689089627), commit `e316c56`).
- [ ] Confirm the material showcase still renders correctly on a physical GPU.

## Validation record

- Commit: `e316c56f5b6517bf62e09e8de5a06dbb8e8e270c`.
- CI run: [Rust CI #547](https://github.com/vivobook244/zyne-engine/actions/runs/44689089627) — successful on `ubuntu-latest` and `windows-latest`.
- Both jobs completed metadata validation, formatting, strict Clippy, tests, workspace check, and the headless lifecycle smoke example.
- Physical-GPU material showcase validation remains open and must be recorded after a local run.

## Color pipeline contract

The scene shader writes linear RGB values into the floating-point HDR target. The presentation shader currently copies those values without tone mapping. The selected window surface prefers an sRGB format, which performs the display transfer at the surface attachment. On an SDR surface, values above the display range are clipped during presentation; they remain preserved in the HDR target for Phase 5.5.2.

## Deferred to later subphases

- Tone mapping and HDR-to-SDR display mapping: Phase 5.5.2.
- Exposure controls: Phase 5.5.3.
- Broader material, lighting, shadow, and presentation integration: Phase 5.5.4.
- Visual comparison, image-based regression checks, and cross-platform visual validation: Phase 5.5.5.
