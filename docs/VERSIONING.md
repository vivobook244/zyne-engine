# Versioning and Release Policy

ZYNE uses Semantic Versioning for the engine workspace and release tags.

## Before 1.0

- The workspace packages share one version, declared in the root Cargo.toml.
- The current development baseline is 0.1.0.
- Public APIs are experimental during 0.x. Breaking API changes may be introduced in a new minor version (for example, 0.1 to 0.2).
- Patch releases (0.x.y) should remain backward-compatible and contain fixes or non-breaking improvements.
- Changes to serialized project/scene formats must be called out explicitly. Their compatibility is not guaranteed until a format versioning policy is introduced.

## From 1.0 onward

- Follow Semantic Versioning: major for breaking changes, minor for backward-compatible functionality, patch for backward-compatible fixes.
- Public API compatibility is evaluated against documented APIs, not internal modules.
- Deprecate public APIs before removal where practical.
- A breaking change to a persisted project format must include migration guidance or a versioned migration path.

## Release process

- Keep all workspace package versions aligned.
- Update the workspace version and `EngineVersion::CURRENT` together.
- Add a release note summarizing features, fixes, known limitations, and migration notes.
- Create an annotated Git tag in the form `vMAJOR.MINOR.PATCH`.
- Run formatting, Clippy, tests, workspace check, and the headless smoke example before tagging.
- Do not publish a release solely because the version number changed; the release must have a passing CI run.

## Stability boundary

ZYNE 0.x does not promise stable renderer, asset, scene, or editor APIs. Internal implementation modules may change without compatibility guarantees. The intended public facade is `zyne-runtime`, with foundational contracts in `zyne-core`.
