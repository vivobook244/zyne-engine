# Engine Configuration

ZYNE project configuration uses TOML. The Phase 0 schema intentionally contains only engine-level settings; window, renderer, asset, and gameplay settings belong to later phases and will be added when their owning subsystems exist.

## Initial schema

~~~toml
application_name = "My ZYNE Game"
fixed_timestep_hz = 60
max_frame_delta_seconds = 0.25
~~~

| Field | Type | Default | Validation |
|---|---|---:|---|
| application_name | String | ZYNE Application | Must not be empty or whitespace |
| fixed_timestep_hz | Integer | 60 | 1 through 1000 |
| max_frame_delta_seconds | Float | 0.25 | Finite and greater than 0, up to 1 second |

Missing fields use defaults. Unknown fields are currently accepted by the TOML deserializer; strict unknown-field rejection can be introduced when the project-file compatibility policy is finalized.

## API

- `EngineConfig::default()` creates the default settings.
- `EngineConfig::from_toml(&str)` parses and validates a TOML document.
- `EngineConfig::to_toml()` validates and serializes the configuration.
- `EngineContext::new(config)` validates the configuration before making it available to engine callbacks.

Parsing and validation failures are returned as `ZyneError`; configuration errors are never silently ignored.

## Ownership boundary

Core owns engine-wide configuration only. Platform owns window and input settings, graphics owns renderer settings, and game projects own game-specific settings. Runtime may compose these settings without moving subsystem-specific policy into core.
