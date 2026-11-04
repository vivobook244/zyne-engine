# Phase 1.1 — Architecture Review and Decisions

**Status:** REVIEWED — READY FOR PHASE 1.2  
**Review date:** 2026-11-04  
**Repository:** `vivobook244/zyne-engine`  
**Branch:** `main`

## 1. Review outcome

The platform architecture defined in [Platform Architecture](../architecture/PLATFORM.md) has been reviewed against the current workspace structure and the selected windowing candidate.

The existing crate boundaries are suitable for Phase 1. The current dependency direction already matches the intended architecture:

- `zyne-runtime` composes `zyne-core`, `zyne-platform`, and `zyne-graphics`.
- `zyne-platform` depends on `zyne-core`, not runtime or graphics.
- `zyne-core` has no workspace-crate dependencies.

No dependency cycle or crate restructuring is required for Phase 1.1.

## 2. Backend decision

**Decision: use stable `winit 0.30.13` as the initial desktop window/event backend in Phase 1.2.**

Evidence reviewed on 2026-11-04:

- The docs.rs release listing identifies 0.30.13 as the latest stable release and 0.31.0-beta.4 as a prerelease.
- The 0.30.13 crate documentation states an MSRV policy of Rust 1.70.
- The ZYNE workspace declares `rust-version = "1.85"`, so 0.30.13 is compatible with the declared minimum.
- The 0.30.13 default feature set includes X11 and Wayland support, covering the two principal Linux desktop backends, alongside Windows support.
- The 0.30 API uses `ApplicationHandler` and `EventLoop::run_app`; window creation is performed from an active event-loop callback such as `resumed`.

References:

- [winit 0.30.13 crate and release metadata](https://docs.rs/crate/winit/0.30.13)
- [winit 0.30.13 ApplicationHandler](https://docs.rs/winit/0.30.13/winit/application/trait.ApplicationHandler.html)
- [winit feature flags](https://docs.rs/crate/winit/0.30.13/features)
- [winit changelog](https://rust-windowing.github.io/winit/winit/changelog/v0_30/index.html)

### Why not 0.31 beta?

The 0.31.0-beta.4 release is prerelease software and changes important integration APIs, including making `ActiveEventLoop` and `Window` traits, returning boxed window trait objects, changing user-event wake-up behavior, and raising the MSRV to 1.85. These changes are not a reason to adopt a beta for the initial ZYNE desktop host.

Re-evaluate the stable 0.31 release when it is available and before a later dependency upgrade. Do not use a beta release in the initial Phase 1 implementation.

### Feature policy

For the first Windows/Linux desktop implementation, use the default 0.30.13 feature set unless Phase 1.2 validation identifies a concrete reason to disable a backend. This keeps both Wayland and X11 available on Linux and enables the default raw-window-handle 0.6 integration for the later graphics phase.

Do not add platform-specific optional features without a demonstrated requirement.

## 3. Platform API design direction

The public API must be ZYNE-owned. The following is the intended conceptual surface, not final Rust signatures:

- **Window configuration:** title, initial logical size, resizable flag, visibility, and window mode. Validate dimensions before passing them to the backend.
- **Window identity:** an opaque engine-owned identifier. Do not expose `winit::window::WindowId` as a core contract.
- **Platform events:** engine-owned events for lifecycle, window state, and input. Translate backend event types at the platform boundary.
- **Platform control:** a narrow control interface for operations that must be requested while dispatching an event, such as requesting application exit or creating a window when the backend permits it.
- **Platform runner:** owns the native event loop and backend state; invokes a handler/callback supplied by the runtime.
- **Platform errors:** typed platform errors with operation context. Convert `winit` errors inside `zyne-platform`.

The final type names, visibility, ownership, and callback signatures must be settled while implementing the first vertical slice in Phase 1.2. Avoid building a broad generic abstraction before there is a working window.

## 4. Runtime integration contract

The intended flow is:

1. The executable initializes logging and creates the engine context.
2. Runtime constructs its application host and provides its platform event handler.
3. Platform creates and owns the winit event loop.
4. The backend reports lifecycle and window events to the platform adapter.
5. Platform translates backend events into ZYNE-owned events.
6. Runtime applies engine policy: engine startup, update scheduling, close handling, and shutdown.
7. Runtime controls frame timing; platform does not implement fixed-timestep policy.
8. Platform exits the native event loop after runtime requests termination and completes backend cleanup.

The backend adapter must support deferred window creation from an active event-loop callback. The runtime must not need to import winit types.

## 5. API and module boundaries

Suggested module layout as features are implemented:

~~~text
crates/zyne-platform/src/
  lib.rs
  error.rs
  application.rs
  window.rs
  event.rs
  input.rs
  backend/
    mod.rs
    winit.rs
~~~

This is a target organization, not a requirement to create empty modules now. Keep backend implementation private. Add modules only when their corresponding feature is implemented.

The runtime may add focused host/runner modules when Phase 1.5 is reached. Do not place runtime lifecycle policy inside the platform crate.

## 6. Decisions explicitly deferred

- Exact public Rust API and callback signatures.
- Window configuration field types and defaults.
- Multi-window support versus a single-window first milestone.
- Event queue versus direct callback dispatch.
- Input state storage and key-code mapping.
- Control-flow strategy and frame pacing.
- Native window handle exposure for wgpu in Phase 2.
- Cross-platform CI matrix details.

These decisions need a working vertical slice or targeted tests; they should not be guessed in an architecture-only review.

## 7. Acceptance review

- [x] Existing crate graph reviewed.
- [x] Core/platform/runtime/graphics ownership reviewed.
- [x] Event-loop and engine lifecycle ownership clarified.
- [x] Public API/backend separation specified.
- [x] Stable backend candidate evaluated.
- [x] MSRV compatibility checked against workspace policy.
- [x] Windows/Linux backend feature strategy recorded.
- [x] Prerelease alternative rejected for initial implementation.
- [x] API design direction recorded without premature implementation.
- [x] Deferred decisions recorded.
- [x] No code or dependency changes made as part of this review.

## 8. Phase decision

**Phase 1.1 is complete.** The architecture is reviewed and ready to guide Phase 1.2 — Window Management.

Phase 1.2 may now add `winit = "0.30.13"` to `zyne-platform`, implement the first window lifecycle, and validate the actual API with Windows/Linux builds. This review itself does not add that dependency.
