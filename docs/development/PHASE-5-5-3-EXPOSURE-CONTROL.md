# Phase 5.5.3 — Exposure Control

**Status:** IMPLEMENTED — CI VALIDATION PENDING  
**Branch:** `main`  
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
- [ ] Run formatting, strict Clippy, locked workspace tests/checks, and Windows/Ubuntu CI.
- [ ] Verify visible exposure changes on a physical GPU.
- [ ] Record visual observations for negative, neutral, and positive EV settings.

## Public API

```rust
graphics.set_exposure_ev(0.0)?;  // neutral exposure
graphics.set_exposure_ev(1.0)?;  // twice the scene-linear brightness
graphics.set_exposure_ev(-1.0)?; // half the scene-linear brightness

let current_ev = graphics.exposure_ev();
```

The setter returns `GraphicsError::InvalidExposure` when the input is non-finite or outside the supported range. The default is 0 EV.

## Validation contract

- EV conversion uses powers of two, so each +1 EV doubles scene-linear exposure and each -1 EV halves it.
- Exposure is applied before tone mapping, preserving the HDR-to-SDR pipeline order.
- The uniform layout is 16-byte aligned and shader validation is covered by the existing Naga WGSL test.
- Resizing recreates the HDR texture bind group while retaining the presentation pipeline's exposure uniform buffer and current setting.

## Deferred work

Automatic exposure measurement, temporal adaptation, exposure compensation from camera metadata, and per-camera exposure are not part of this milestone.
