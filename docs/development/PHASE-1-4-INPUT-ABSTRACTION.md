# Phase 1.4 — Input Abstraction

**Status:** IMPLEMENTATION IN PROGRESS  
**Repository:** `vivobook244/zyne-engine`  
**Branch:** `main`

## Goal

Expose portable keyboard and mouse input events and maintain per-frame input state without leaking winit types into runtime or game code.

## Ownership

- `zyne-platform` translates native winit keyboard/mouse events into ZYNE-owned `InputEvent`.
- `zyne-runtime` owns `InputState` and its per-frame transition state.
- Game code queries runtime input state using ZYNE-owned key and mouse types.
- Focus loss resets held buttons to avoid stuck keys/buttons.

## Implemented

- [x] Portable `KeyCode`, `MouseButton`, `ButtonState`, `ScrollDelta`, and `InputEvent` contracts.
- [x] Runtime `InputState` with held, just-pressed, and just-released queries.
- [x] Cursor position and per-frame scroll accumulation.
- [x] Translate winit keyboard, cursor, mouse-button, and wheel events.
- [x] Reset input state on focus loss through runtime event processing.
- [x] Unit tests for keyboard transitions, key repeat, mouse state, cursor, scroll, and focus loss.

## Frame contract

Call `InputState::begin_frame()` once at the beginning of each frame, then pass received events to `process_platform_event`. `just_key_pressed` and `just_key_released` (and mouse equivalents) describe transitions during the current frame. Held state persists until release. Scroll values accumulate until the next `begin_frame`.

The current portable key enum covers common gameplay and application keys. Unmapped physical keys are ignored for now; expand the public enum when a concrete use case requires them.

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo check --locked --workspace
cargo run --locked -p zyne-platform --example window_smoke
```

Manual test: press/release WASD and Space, click left/right mouse buttons, move the cursor, and scroll. Confirm event callbacks receive `PlatformEvent::Input` values.
