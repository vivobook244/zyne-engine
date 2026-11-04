# Platform Architecture

**Status:** REVIEWED — READY FOR PHASE 1.2  
**Scope:** Architecture contract; no window backend implementation yet.

## 1. Purpose

The `zyne-platform` crate is the boundary between ZYNE and the host operating system. It provides OS-facing capabilities through engine-owned APIs while keeping operating-system and third-party windowing types out of `zyne-core` and out of unrelated engine subsystems.

Phase 1.1 establishes ownership and dependency rules. Window creation, concrete event translation, and input implementation are subsequent Phase 1 work.

## 2. Responsibility boundaries

| Crate | Owns | Does not own |
|---|---|---|
| `zyne-core` | Engine-wide contracts, shared configuration, context, version, and common errors | OS handles, event loops, windows, input devices |
| `zyne-platform` | OS integration, native window lifecycle, OS event acquisition/translation, platform input acquisition, platform-specific errors | Engine/game rules, frame policy, rendering, editor UI |
| `zyne-runtime` | Application host, engine lifecycle policy, update scheduling, frame-time policy, coordination of subsystems | Native OS implementation, window backend internals |
| `zyne-graphics` | GPU resources, rendering and presentation | Window/event-loop ownership, gameplay policy |

The platform crate may depend on core contracts. Runtime composes core, platform, and graphics. Core must never depend on platform.

## 3. Event-loop ownership

The selected window backend (planned: `winit`) must run its event loop according to the host OS requirements, including main-thread constraints where applicable.

Ownership is split deliberately:

- **Platform** owns backend integration: creating the event loop, receiving native events, translating them into engine-owned platform events, and exposing the backend's required control-flow mechanism.
- **Runtime** owns engine policy: when startup occurs, when engine updates are called, how close requests affect shutdown, how frame timing is calculated, and how shutdown is completed.
- **Core** owns only backend-independent engine contracts. It does not know that an event loop or window exists.

The runtime may provide callbacks or a host state object consumed by the platform event-loop adapter. The exact Rust API and callback/lifetime shape will be selected during implementation after validating the chosen backend version. Do not force a generic async event loop or move the OS event loop into core.

## 4. Public API and backend separation

The platform crate should expose a small, deliberate public API from its crate root.

- Public engine-facing types belong in focused modules such as application, window, event, and input as those capabilities are implemented.
- Backend-specific implementation belongs in private modules (for example, a future `backend::winit` module).
- Third-party types such as `winit::window::Window`, native handles, and backend event enums must not leak into the public engine API unless a future documented interoperability requirement justifies it.
- Public platform events and window configuration must use ZYNE-owned types.
- Do not add placeholder abstractions for unimplemented backends or features.

A future backend replacement should primarily affect private platform implementation and its adapter, not core or gameplay code.

## 5. Dependency direction

Allowed workspace dependency direction:

~~~text
zyne-runtime
  |---> zyne-core
  |---> zyne-platform ---> zyne-core
  |---> zyne-graphics ---> zyne-core
                         ---> zyne-math

zyne-core, zyne-math: no workspace-crate dependencies
zyne-platform: no dependency on runtime, graphics, gameplay, or editor
zyne-graphics: no dependency on platform, runtime, gameplay, or editor
~~~

Rules:

1. No circular crate dependencies.
2. `zyne-platform` must not depend on `zyne-runtime`.
3. `zyne-core` must not import OS or backend types.
4. Graphics must not own window creation or the OS event loop. It may receive a presentation target/surface through an explicit integration boundary in Phase 2.
5. Runtime is the composition layer; it should not reimplement platform backend functionality.

## 6. Error boundaries

- Platform operations should return a platform-owned typed error (for example, a future `PlatformError`) with useful operation context.
- Third-party backend errors should be converted inside `zyne-platform`; backend error types should not become public core contracts.
- Runtime maps platform failures into the engine-level error/reporting path when crossing the runtime boundary.
- Recoverable user input and normal close requests are events, not errors.
- Avoid panics for normal OS failures such as window creation failure.

The exact error variants will be defined alongside the first platform operations, rather than inventing unused variants in this architecture-only step.

## 7. Configuration ownership

- `zyne-core` continues to own engine-wide configuration and validation.
- Window-specific options (initial dimensions, title, resizability, window mode) belong to platform-owned configuration types.
- Runtime combines project/engine configuration with platform configuration when constructing the application host.
- Renderer preferences such as VSync may be represented in project configuration, but their behavior belongs to the graphics/presentation integration in Phase 2.
- Do not make core depend on platform merely to store window configuration. If project-level serialization needs window settings, use a backend-independent configuration contract with an intentional ownership decision before implementation.

## 8. Timing and frame policy

The platform backend supplies event-loop opportunities and timestamps only where its API requires them. Runtime owns:

- frame delta calculation and clamping;
- fixed-timestep accumulator and update frequency;
- variable update scheduling;
- frame/FPS diagnostics policy.

Use `std::time` initially unless implementation demonstrates a need for another timing dependency. Rendering/presentation pacing and VSync integration are addressed in Phase 2.

## 9. Threading and resource lifetime

- Respect the selected backend's thread-affinity and event-loop requirements.
- Do not assume that native windows or event-loop handles are freely transferable between threads.
- Keep native handles private to platform/graphics integration boundaries.
- Define explicit ownership and shutdown order for event loop, windows, and future presentation resources during implementation.
- Do not introduce background threads solely to make the API appear asynchronous.

## 10. Backend decision

**Selected initial backend: stable `winit 0.30.13`.** The release and API were reviewed on 2026-11-04. It is compatible with the workspace's declared Rust 1.85 minimum (winit documents MSRV 1.70), and its default feature set includes Linux X11 and Wayland support. Its `ApplicationHandler` / `run_app` event-loop model requires window creation from an active event-loop callback such as `resumed`.

Do not use `0.31.0-beta.4` for the initial implementation: it is prerelease and contains significant API changes. Re-evaluate the stable 0.31 release before a future upgrade.

For the first Windows/Linux implementation, retain winit's default features unless a concrete Phase 1.2 validation result justifies changing them. The detailed review and references are recorded in [Phase 1.1 Review](../development/PHASE-1-1-REVIEW.md).

The dependency is not added by this architecture document; it may be added in Phase 1.2 when window implementation begins.

## 11. Phase 1.1 acceptance criteria

- [x] Responsibilities of core, platform, runtime, and graphics are explicit.
- [x] Platform/backend API separation is documented.
- [x] Event-loop mechanics and engine lifecycle policy have distinct owners.
- [x] Dependency direction and forbidden dependencies are documented.
- [x] Error conversion boundary is documented.
- [x] Timing/frame policy is assigned to runtime.
- [x] Window and renderer configuration ownership is documented.
- [x] Architecture overview is aligned with this document.
- [x] Backend candidate and API constraints reviewed before Phase 1.2.
- [x] Review record published in [Phase 1.1 Review](../development/PHASE-1-1-REVIEW.md).

## 12. Deferred to later work

- Concrete `winit` version and feature selection.
- Public Rust type signatures and callback/lifetime design.
- Window creation and native handle access.
- Concrete platform event and input types.
- Runtime event-loop integration and shutdown implementation.
- Graphics surface creation and presentation integration.
- Cross-platform runtime tests.

These are intentionally deferred; Phase 1.1 is an architecture contract, not a claim that desktop windowing is already implemented.
