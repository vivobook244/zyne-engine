# Phase 5.5.3 — Exposure Control

**Status:** COMPLETE — AUTOMATED AND PHYSICAL GPU VALIDATION PASSED  
**Branch:** `main`  
**Implementation commit:** `57062b12`  
**Interactive showcase commit:** `fde31713`  
**Prerequisite:** Phase 5.5.2 — Tone Mapping  
**Goal:** Provide deterministic exposure compensation in photographic EV stops at the final presentation stage, without coupling exposure to materials, lights, or scene data.

## Architecture decisions

- Exposure is a presentation setting owned by `PresentationPipeline`.
- The public renderer API accepts exposure compensation in EV stops through `GraphicsContext::set_exposure_ev`.
- Zero EV is neutral; positive EV brightens and negative EV darkens.
- Convert EV to a linear multiplier in WGSL using `exp2(EV)`.
- Apply exposure to scene-linear HDR RGB immediately before the fitted ACES tone-mapping operator.
- Restrict values to finite values in the inclusive range -16 to +16 EV.
- Store exposure in a 16-byte uniform buffer and update it without recreating the render pipeline or bind group.
- Default exposure is 0 EV.
- Automatic exposure, luminance metering, adaptation over time, and camera exposure metadata are explicitly out of scope.

## Implementation checklist

- [x] Add presentation exposure uniform data.
- [x] Apply `2^EV` exposure scaling before tone mapping.
- [x] Add `GraphicsContext::set_exposure_ev` and `GraphicsContext::exposure_ev`.
- [x] Reject non-finite and out-of-range EV values.
- [x] Add boundary and invalid-value unit tests.
- [x] Keep exposure independent from material and lighting APIs.
- [x] Run formatting, strict Clippy, locked workspace tests/checks, and Windows/Ubuntu CI (CI run [#577](https://github.com/vivobook244/zyne-engine/actions/runs/44710864606), commit `57062b12`).
- [x] Add interactive exposure controls to `material_showcase` using Arrow Up, Arrow Down, and Digit0.
- [x] Apply the engine's optional exposure override through the runtime without coupling the core crate to graphics.
- [x] Verify visible exposure changes on a physical GPU; the user confirmed all three showcase controls behave as intended.
- [x] Verify resizing preserves the current exposure value; the user confirmed the value remains unchanged after resizing.
- [x] Run automated CI for the interactive showcase/runtime integration on Ubuntu and Windows (CI run [#578](https://github.com/vivobook244/zyne-engine/actions/runs/44712701063), commit `fde31713`).

## Public API

```rust
graphics.set_exposure_ev(0.0)?;  // neutral exposure
graphics.set_exposure_ev(1.0)?;  // twice the scene-linear brightness
graphics.set_exposure_ev(-1.0)?; // half the scene-linear brightness

let current_ev = graphics.exposure_ev();
```

The setter returns `GraphicsError::InvalidExposure` when the input is non-finite or outside the supported range. The default is 0 EV.

## Showcase controls

The `material_showcase` example exposes a runtime-adjustable range from -4 to +4 EV:

- Arrow Up: increase exposure by 0.5 EV.
- Arrow Down: decrease exposure by 0.5 EV.
- Digit0: reset exposure to 0 EV.

The runtime applies the optional `Engine::presentation_exposure_ev` override before rendering. Engines that return `None` retain the current graphics exposure setting.

## Validation contract

- EV conversion uses powers of two, so each +1 EV doubles scene-linear exposure and each -1 EV halves it.
- Exposure is applied before tone mapping, preserving the HDR-to-SDR pipeline order.
- The uniform layout is 16-byte aligned and shader validation is covered by the existing Naga WGSL test.
- Resizing recreates the HDR texture bind group while retaining the presentation pipeline's exposure uniform buffer and current setting.
- Physical GPU validation confirmed interactive exposure changes and persistence across window resize.

## Deferred work

Automatic exposure measurement, temporal adaptation, exposure compensation from camera metadata, and per-camera exposure are not part of this milestone.
