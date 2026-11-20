# Lighting Guide

This document describes the lighting interfaces currently implemented by ZYNE. The renderer uses forward rendering with a Blinn–Phong lighting model. This is the current engine contract, not a promise of features that are still planned.

## Overview

ZYNE supports three direct-light types and ambient illumination:

- **Directional light** — light rays share one direction, suitable for sunlight or moonlight.
- **Point light** — light emitted from an entity position in all directions, suitable for lamps and small local sources.
- **Spot light** — a point-like source restricted by an inner and outer cone, suitable for flashlights and focused fixtures.
- **Ambient color** — a constant ambient contribution applied with the material ambient term.

Lighting components are defined in `zyne-scene`, stored on scene entities, and serialized as part of scene documents. The renderer consumes light snapshots through its lighting data contract.

## Creating lights

Import the scene light types and attach them to entities. A light constructor validates its parameters and returns `Result<_, LightError>`.

```rust
use zyne_math::Transform3d;
use zyne_scene::{DirectionalLight, PointLight, Scene, SpotLight};

fn add_lights(scene: &mut Scene) -> Result<(), Box<dyn std::error::Error>> {
    // Direction points along the light rays.
    let sun_entity = scene.spawn_entity("Sun", Transform3d::identity())?;
    let sun = DirectionalLight::new(
        [0.35, -0.8, -0.55],
        [1.0, 0.96, 0.88],
        2.2,
    )?;
    assert!(scene.set_entity_directional_light(sun_entity, Some(sun)));

    // Point and spot light positions come from their owning entity transforms.
    let lamp_entity = scene.spawn_entity(
        "Lamp",
        Transform3d::new(
            [2.0, 1.5, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            [1.0; 3],
        )?,
    )?;
    let lamp = PointLight::new([1.0, 0.72, 0.42], 8.0, 10.0)?;
    assert!(scene.set_entity_point_light(lamp_entity, Some(lamp)));

    let flashlight_entity = scene.spawn_entity(
        "Flashlight",
        Transform3d::new(
            [0.0, 1.6, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            [1.0; 3],
        )?,
    )?;
    let flashlight = SpotLight::new(
        [0.0, 0.0, -1.0],
        [0.85, 0.92, 1.0],
        12.0,
        18.0,
        15.0_f32.to_radians(),
        28.0_f32.to_radians(),
    )?;
    assert!(scene.set_entity_spot_light(flashlight_entity, Some(flashlight)));

    Ok(())
}
```

The `set_entity_*_light` methods return `false` if the entity ID is not present in the scene. Pass `None` to remove a light component. The corresponding `entity_*_light` methods return the component when present.

Scene documents preserve directional, point, and spot light components during RON serialization and deserialization.

## Parameters and validation

All light colors are RGB values. Scene light constructors require finite, non-negative color components and finite, non-negative intensity. Color values above `1.0` are allowed; this is useful for brighter sources and future HDR workflows.

| Light | Parameters | Rules |
|---|---|---|
| Directional | direction, color, intensity | Direction must be finite and non-zero; it is normalized by the constructor. |
| Point | color, intensity, range | Range must be finite and greater than zero. Position is supplied by the owning entity transform. |
| Spot | direction, color, intensity, range, inner angle, outer angle | Direction must be finite and non-zero; range must be positive; angles are radians and must satisfy `0 <= inner <= outer <= 90°`. |

A spot light direction is normalized by its constructor. The inner and outer cone angles are stored in radians; the shader compares their cosine values when evaluating the cone.

## Renderer limits

The current GPU lighting uniform and WGSL shader support these maximum counts per frame:

| Light type | Maximum uploaded |
|---|---:|
| Directional | 4 |
| Point | 16 |
| Spot | 8 |

The renderer uploads no more than the first supported number of each type. Scene entities are extracted in scene creation order, and that order is preserved when a scene is serialized and loaded again. Keep important lights early in the scene creation order so they remain within the supported range. Invalid light data is skipped before GPU upload.

The current renderer uses **one directional shadow map**:

- The first valid directional light in scene creation order accepted for GPU upload owns the shadow map.
- Other directional lights still contribute diffuse and specular illumination, but are unshadowed.
- Point and spot lights contribute illumination but do not cast shadows in the current renderer.

The CPU shadow-camera selection and GPU directional-light upload share the same validity rule so that the shadow camera follows the directional light that actually owns the first uploaded slot.

## Shading behavior

The current shader combines:

1. Ambient material contribution multiplied by scene ambient color.
2. Directional Blinn–Phong diffuse and specular contributions.
3. Point-light contributions with range-based attenuation.
4. Spot-light contributions with range attenuation and a smoothed cone factor.

Directional light direction describes the direction of the light rays; the shader uses its negated direction to calculate the direction from the surface toward the light. Point and spot positions are taken from the owning entity's world-space position in the renderer snapshot.

Shadow visibility is evaluated for the first uploaded directional light. The current implementation uses a depth shadow map with 3×3 percentage-closer filtering (PCF), receiver depth bias, and a scene-fitted, texel-stabilized directional shadow camera.

## Practical setup

- Start with one directional key light and a modest ambient color.
- Add point lights for local pools of light; keep their ranges appropriate to the scene scale.
- Use spot lights for focused beams. Keep the inner cone narrower than the outer cone for a soft edge.
- Create the primary shadow-casting directional light before fill lights; scene creation order determines directional-light upload order and shadow-map ownership.
- Keep the number of active lights within the renderer limits.
- Avoid adding many high-intensity lights before checking material response; Blinn–Phong contributions are additive.

## Current scope and non-goals

The current lighting implementation is Blinn–Phong forward shading with one directional shadow map. The renderer now writes scene-linear lighting into an RGBA16Float HDR target and applies a fitted ACES tone-mapping curve during final presentation. This does not imply support for physically based rendering (PBR), multiple shadow maps, point-light shadows, spot-light shadows, image-based lighting, or automatic exposure adaptation. Fixed EV exposure compensation is available in the presentation pass; automatic exposure adaptation remains future work.

## Related implementation

- `crates/zyne-scene/src/lighting.rs` — validated scene light components.
- `crates/zyne-scene/src/world.rs` — scene entity light attachment and scene document persistence.
- `crates/zyne-graphics/src/lighting.rs` — renderer light snapshots and capacity limits.
- `crates/zyne-graphics/src/shaders/basic_3d.wgsl` — current Blinn–Phong and shadow shader.
- `crates/zyne-graphics/src/pipeline.rs` — GPU uniform upload and directional shadow camera.
