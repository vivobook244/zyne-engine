# Phase 1.5 — Application Lifecycle

**Status:** DESIGN REVIEW — READY FOR IMPLEMENTATION  
**Branch:** `main`  
**Scope:** Runtime-owned application orchestration over the existing platform window runner. This document defines the implementation contract; it does not claim that the application loop is implemented.

## 1. Goal

Provide one public runtime entry point that starts a ZYNE desktop application, owns the engine lifecycle, processes platform events, maintains per-frame input state, schedules updates, and performs graceful shutdown.

The first vertical slice remains intentionally small: one window, one engine instance, one event-loop thread, no renderer integration, and no background runtime threads.

## 2. Current foundation

The following pieces already exist:

- `zyne_core::Engine`: `on_start`, variable-rate `on_update`, and `on_shutdown`.
- `EngineConfig` and validated immutable `EngineContext`.
- `zyne_platform::WindowRunner`: owns the winit 0.30 event loop and native window.
- `PlatformEvent`: lifecycle, window, and normalized input events.
- `zyne_runtime::EventQueue`: FIFO event storage.
- `zyne_runtime::InputState`: held/pressed/released keyboard and mouse state, cursor position, and accumulated scroll.
- Runtime diagnostics initialization.

The missing piece is a runtime host that connects these contracts. The current headless smoke example manually invokes engine callbacks and is not a desktop application loop.

## 3. Ownership contract

| Layer | Responsibility |
|---|---|
| `zyne-core` | Engine lifecycle contract, engine configuration/context, engine-wide errors. No OS or window types. |
| `zyne-platform` | Native event loop, window creation, OS event translation, event-loop wake/sleep mechanics. No engine update policy. |
| `zyne-runtime` | Public application entry point, engine lifecycle orchestration, event queue/input processing, frame clock, fixed-step accumulator, shutdown policy. |
| `zyne-graphics` | No Phase 1.5 lifecycle ownership. Renderer/surface integration is Phase 2. |

Dependency direction remains unchanged. Platform must not depend on runtime. Core must not depend on platform.

## 4. Public API direction

Implement a runtime-owned application host. The public shape should be equivalent to:

- `ApplicationConfig`: combines validated `EngineConfig` and `WindowConfig`; defaults should produce a 1280×720 window and use the application name as the window title unless explicitly overridden.
- `Application`: owns the configuration and starts one supplied engine through a consuming `run(engine)` operation.
- `ApplicationRunner` / private host state: adapts `WindowEventHandler`, owns the engine, `EngineContext`, `EventQueue`, `InputState`, and timing state for the duration of the event loop.

The final Rust signatures must respect the actual winit callback and ownership constraints. Keep winit types private. Do not expose native handles through `zyne-runtime`.

The existing `Engine` trait remains backend-independent. Do not make `zyne-core` depend on `zyne-platform` merely to expose input.

## 5. Lifecycle contract

Normal successful startup and shutdown order:

1. Validate application and window configuration.
2. Construct `EngineContext`.
3. Initialize runtime-owned state (event queue, input state, clock).
4. Create the native window through `WindowRunner`.
5. Call `Engine::on_start` exactly once when the application is ready.
6. Process platform events and run scheduled engine updates.
7. On a normal close request, stop scheduling new updates.
8. Call `Engine::on_shutdown` exactly once if and only if `on_start` completed successfully.
9. Return the result to the caller.

Error policy:

- If configuration validation or window creation fails before engine startup, do not call `on_shutdown`.
- If `on_start` fails, do not call `on_shutdown`; startup did not complete.
- If an update fails after successful startup, request loop exit, attempt `on_shutdown` once, and return the original update error. If shutdown also fails, log the shutdown error without replacing the primary failure.
- On a normal close, return a shutdown error if `on_shutdown` fails.
- A platform callback failure must exit the loop. If the engine has started, still attempt graceful shutdown.
- Never invoke shutdown twice, including when both close and an error are observed.
- A forced process termination or OS kill cannot guarantee graceful callbacks; document this limitation.

Avoid panics for expected platform and engine failures. Convert platform errors to the existing engine-level error path at the runtime boundary, preserving useful context.

## 6. Event and input processing

The platform callback is the event producer; runtime is the consumer/policy owner.

- Preserve arrival order when transferring events into `EventQueue`.
- Drain queued events at a deterministic frame boundary.
- Feed every drained event to `InputState::process_platform_event`.
- Focus loss must release held keys/buttons through the existing `InputState` behavior.
- Clear transient pressed/released/scroll values once at the beginning of a new frame, not once per individual event.
- Do not silently discard close, focus, resize, or lifecycle events.
- The engine must remain independent of winit and native event enums.

**Input exposure decision is intentionally constrained:** Phase 1.5 must make the runtime input state available to runtime-level code and tests, but must not add platform dependencies to `zyne-core`. Before game-facing input callbacks are implemented, review whether the stable input value types belong in core or in a dedicated engine-facing contract crate. Do not leak `zyne_platform::InputEvent` into the core `Engine` trait.

## 7. Timing and update scheduling

Runtime owns time measurement using `std::time::Instant`.

- Clamp measured frame delta to `EngineConfig::max_frame_delta_seconds`.
- Use `fixed_timestep_hz` to calculate a fixed-step duration and maintain an accumulator.
- Execute zero or more fixed updates per frame, with a documented catch-up ceiling to avoid a spiral of death.
- Preserve the existing variable-rate `Engine::on_update` contract for the frame update.
- Do not pass an unclamped delta to engine callbacks.
- Do not claim a 60 FPS presentation guarantee. VSync and render pacing belong to Phase 2.
- Avoid a busy loop. The platform adapter must provide a wake-up path suitable for runtime's next update deadline; the exact callback/control-flow API must be validated against winit 0.30.13 during implementation.

**Compatibility requirement:** If a fixed-update callback is added to `Engine`, it must have a default no-op implementation so existing engine implementations continue to compile. Decide and test the precise fixed/variable callback order before implementation. Do not silently reinterpret the existing `on_update` as fixed-rate.

## 8. Window and lifecycle events

- `Resumed`: mark the application active and allow window-dependent work.
- `Suspended`: pause or defer window-dependent work; do not destroy engine state in this phase.
- `Focused(false)`: clear held input state.
- `Resized` and `ScaleFactorChanged`: retain the latest window metrics for future renderer integration; do not add GPU surface code here.
- `CloseRequested`: begin orderly shutdown.
- `RedrawRequested`: reserved as the presentation boundary; Phase 1.5 may use it to drive a smoke update, but actual rendering is Phase 2.

A close request must not be treated as an engine error.

## 9. Implementation sequence

1. **API and configuration:** add runtime-owned `ApplicationConfig` with validation and tests; avoid changing core/platform ownership.
2. **Platform scheduling hook:** extend the platform callback contract only as needed to let runtime request the next wake-up deadline without exposing winit types. Validate the design against winit 0.30.13 and avoid a busy poll loop.
3. **Runtime host:** implement the private event handler/host that owns the engine and runtime state.
4. **Lifecycle/error handling:** implement exactly-once startup/shutdown and preserve primary errors.
5. **Frame clock:** implement clamped delta, fixed-step accumulator, catch-up limit, and variable frame update.
6. **Event/input integration:** queue events in order, drain at frame boundaries, update input state, and test focus loss.
7. **Runnable example:** add a desktop application smoke example that opens a window, logs lifecycle/update/input diagnostics, and exits cleanly.
8. **Documentation and validation:** update architecture/roadmap only after implementation and validation.

Do not add rendering, ECS, asset loading, physics, audio, editor UI, async runtime, or extra dependencies in this phase.

## 10. Acceptance criteria

- [ ] A game can start a desktop application through one public `zyne-runtime` API.
- [ ] The runtime owns the engine for the entire event-loop lifetime.
- [ ] `on_start` is called once after successful window/application setup.
- [ ] `on_shutdown` is called exactly once after successful startup on normal exit and recoverable runtime failure.
- [ ] Primary errors are preserved when shutdown also fails.
- [ ] Platform events are processed in FIFO order.
- [ ] Input transient state is reset at frame boundaries; focus loss releases held inputs.
- [ ] Frame delta is clamped and fixed-step catch-up is bounded.
- [ ] The event loop sleeps/wakes without a busy loop.
- [ ] Desktop smoke example opens and closes normally on Windows and Linux.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo clippy --locked --workspace --all-targets -- -D warnings` passes.
- [ ] `cargo test --locked --workspace` passes.
- [ ] `cargo check --locked --workspace` passes.
- [ ] CI passes on supported Windows and Linux runners.

Phase 1.5 is not complete until the acceptance criteria are verified. A successful headless test alone is not sufficient.
