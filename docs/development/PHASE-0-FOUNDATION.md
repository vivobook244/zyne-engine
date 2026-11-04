# Phase 0 — Project Foundation

**Status:** IN PROGRESS  
**Tracking rule:** Mark an item complete only after its evidence exists in the repository or its validation command has passed. A configured CI workflow is not the same as a successful CI run.

## Status summary

- [x] DONE — Initial Cargo workspace and five crate manifests exist.
- [x] DONE — Stable Rust toolchain, workspace edition, package metadata, and baseline lint policy are configured.
- [x] DONE — Initial crate responsibilities and dependency direction are documented.
- [x] DONE — Basic core error/result types exist.
- [x] DONE — Project overview, roadmap, architecture overview, contribution guide, and MIT license exist.
- [x] DONE — GitHub Actions workflow is configured for formatting, Clippy, tests, and workspace check.
- [ ] PARTIAL — Crate boundaries and API conventions are documented; dependency graph still needs Cargo validation.
- [ ] PARTIAL — Coordinate direction is documented; matrix, transform composition, and import conversion remain provisional.
- [x] DONE — Engine lifecycle, context, version, typed errors, and validated configuration contracts are implemented.
- [x] DONE — TOML configuration format, schema, defaults, parser, serializer, and validation are implemented.
- [x] DONE — Runtime logging initialization via tracing-subscriber is implemented; execution still needs validation.
- [x] DONE — UUID-based persistent resource identity and generation-checked typed runtime handles are implemented and documented; full registry/loader remains deferred.
- [x] DONE — Initial public API and module conventions are documented.
- [ ] PARTIAL — Core unit tests are written; integration/smoke tests and execution are still pending.
- [ ] TODO — Minimal runnable example / smoke-test target.
- [x] DONE — Direct dependency inventory and dependency review policy are documented.
- [ ] TODO — Cargo.lock generation and repository tracking (verify during local Cargo validation).
- [ ] TODO — Run and pass all local validation commands.
- [ ] TODO — Verify the GitHub Actions workflow completes successfully.
- [ ] TODO — Define initial versioning and release policy.

## Detailed development rundown

### 0.1 Workspace and toolchain

- [x] Create a single Cargo workspace (no monorepo split and no Docker requirement).
- [x] Configure workspace resolver and shared package metadata.
- [x] Set Rust stable toolchain and required components (rustfmt, clippy).
- [x] Set workspace lint policy (unsafe_code = forbid, unused_must_use = deny, Clippy warnings).
- [ ] Run cargo metadata --no-deps and inspect the resolved workspace.
- [ ] Run cargo check --workspace successfully.
- [ ] Generate and commit Cargo.lock after validation.

**Evidence:** Cargo.toml, rust-toolchain.toml.

### 0.2 Crate structure and dependency graph

- [x] Create zyne-core.
- [x] Create zyne-math.
- [x] Create zyne-platform.
- [x] Create zyne-graphics.
- [x] Create zyne-runtime.
- [x] Add initial path dependencies between crates.
- [ ] Validate the dependency graph with Cargo tooling.
- [ ] Confirm there are no circular dependencies.
- [ ] Document allowed dependency direction and public ownership for every crate.

**Current intended boundaries:**
- Core: shared engine contracts; independent from platform, graphics, gameplay, and editor.
- Math: shared math types and engine coordinate conventions.
- Platform: OS, window, input, and application lifecycle.
- Graphics: GPU and rendering implementation; no gameplay policy.
- Runtime: public facade used by game projects.

**Evidence:** crates/*/Cargo.toml, docs/architecture/OVERVIEW.md.

### 0.3 Core contracts and error handling

- [x] Add ZyneResult<T>.
- [x] Add basic ZyneError variants for invalid input, missing resources, and subsystem failures.
- [x] Define Engine lifecycle contract.
- [x] Define EngineConfig and configuration validation errors.
- [x] Define EngineContext ownership/access rules.
- [x] Define EngineVersion and version exposure.
- [ ] Review error categories and conversions across crate boundaries.
- [ ] Add tests for error formatting and configuration validation (configuration tests are written; execution pending).

**Evidence:** crates/zyne-core/src/lib.rs.

### 0.4 Math and coordinate conventions

- [x] Define the intended right-handed world coordinate system and +Y up.
- [x] Record the intended +Z forward convention.
- [x] Add a preliminary WorldUnit = f32 alias.
- [ ] Specify vector, matrix, quaternion, and transform conventions.
- [ ] Specify local-space versus world-space behavior.
- [ ] Specify matrix multiplication / transform composition order.
- [ ] Specify camera forward and projection conventions.
- [ ] Specify GLTF import coordinate conversion boundary.
- [ ] Add convention tests when concrete math types are introduced.

**Evidence:** crates/zyne-math/src/lib.rs, docs/architecture/OVERVIEW.md.

### 0.5 Configuration

- [x] Choose and document TOML as the engine/project configuration format.
- [x] Define initial configuration fields and defaults.
- [x] Define validation behavior and useful error messages.
- [x] Add TOML parse/serialize and validation unit tests (execution pending).
- [x] Document configuration ownership boundaries.

### 0.6 Logging and diagnostics

- [x] Add tracing and a minimal subscriber setup in the runtime host boundary.
- [x] Define initial log-level and module-target conventions.
- [ ] Define concrete startup/shutdown event coverage as subsystems are implemented.
- [x] Keep subscriber initialization in runtime; library crates do not initialize it.
- [ ] Add a small test or smoke check for diagnostic initialization.

### 0.7 Resource conventions

- [x] Define persistent resource identity (`ResourceId`, UUID v4).
- [x] Define typed runtime handles (`Handle<T>`) and stale-handle detection contract (index + generation validation by registry).
- [x] Define manager ownership and Unloaded/Loading/Loaded/Failed lifecycle rules.
- [x] Define that `ResourceId` is stable across save/load while runtime handles are process-local and must be reacquired.
- [x] Document persisted metadata versus transient GPU/runtime objects.

### 0.8 Public API and coding conventions

- [x] Define public versus internal module policy.
- [x] Define naming, visibility, documentation, and re-export conventions.
- [x] Define typed ZyneResult/ZyneError as the initial cross-crate error contract.
- [x] Define dependency addition policy (feature need, license, maintenance, platform, compile cost).
- [ ] Define compatibility expectations for public APIs before 0.1.

### 0.9 Tests and examples

- [x] Add core unit tests for configuration, context, and version (execution pending).
- [ ] Add integration-test layout and at least one workspace smoke test.
- [ ] Add a minimal example that can be built and run (or a documented headless smoke example if no window is introduced in Phase 0).
- [ ] Ensure examples do not introduce Phase 1/2 runtime dependencies prematurely.

### 0.10 Documentation and project hygiene

- [x] Add project README and development commands.
- [x] Add phase roadmap.
- [x] Add architecture overview and initial crate boundaries.
- [x] Add contribution guide.
- [x] Add MIT license.
- [x] Add Cargo-oriented .gitignore.
- [x] Add dependency inventory with rationale for current direct dependencies.
- [ ] Add remaining error and coordinate convention details; configuration, resource, and general engine conventions are documented.
- [ ] Add a Phase 0 completion record with validation evidence.

### 0.11 CI and validation

- [x] Configure GitHub Actions for cargo fmt, cargo clippy, cargo test, and cargo check.
- [ ] Verify the workflow triggers on the current main branch and completes successfully.
- [ ] Obtain a successful CI run; fix any formatting, lint, test, or compilation failures.
- [ ] Run the same commands locally before closing Phase 0.

**Required validation commands:**

~~~sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace
~~~

## Phase 0 exit criteria

Phase 0 may be marked DONE only when all of the following are true:

- [ ] Workspace metadata resolves and all workspace crates compile.
- [ ] No circular crate dependency exists.
- [x] Core engine contracts, typed errors, and configuration foundation are defined.
- [x] Logging/diagnostics foundation is present.
- [ ] Coordinate, transform, resource, and public API conventions are documented.
- [ ] Tests and a minimal example/smoke target exist and pass.
- [ ] Dependency choices and Cargo.lock are reviewed and committed.
- [ ] Formatting, Clippy, tests, and workspace check pass.
- [ ] GitHub Actions validation is green.
- [ ] README, architecture, contribution guide, and this checklist reflect the actual implementation.

**Current decision:** Phase 0 remains IN PROGRESS. Do not start Phase 1 implementation until the exit criteria above are satisfied or explicitly re-scoped.
