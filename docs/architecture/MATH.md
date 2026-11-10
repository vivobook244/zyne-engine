# Math and Coordinate Conventions

This document fixes the conventions that all ZYNE math, camera, scene, animation, import, and rendering code must follow. Implementations may use `glam`, but public behavior must follow these rules rather than exposing dependency defaults accidentally.

## World coordinate system

- Handedness: right-handed.
- Up: +Y.
- Engine world forward: +Z.
- Right: +X.
- One world unit is a logical distance unit. Its physical scale is project-defined; do not assume one unit equals one meter in generic engine code.
- Angles are expressed in radians unless an API explicitly says otherwise.

A positive rotation follows the right-hand rule around its axis.

## Vectors and points

- Vectors represent directions or displacements and are not translated.
- Points represent positions and are translated by transforms.
- Vector normalization must handle zero-length input explicitly; callers must not assume a zero vector can be normalized.
- World-space and local-space values should be distinguishable by naming or API structure when mixing them could cause ambiguity.

## Matrices and transforms

- Matrices use column-vector algebra: transform a point with `M * p`.
- Matrix memory layout is column-major in the engine-facing math API.
- A local transform is composed as `T * R * S` (scale, then rotate, then translate when applied to a column vector).
- A child world transform is `parent_world * child_local`.
- The local transform maps an object's local coordinates into its parent space; the world transform maps local coordinates into world space.
- Matrix multiplication is not commutative. Do not reorder transform factors.
- Normal vectors must use the appropriate inverse-transpose transform when non-uniform scale is present.

The initial `zyne-math::Transform3d` implementation stores translation, a normalized quaternion, and scale. Quaternion components are ordered `x, y, z, w`. Its constructor and deserializer reject non-finite components and zero-length quaternions. `to_matrix()` returns the column-major local-to-parent matrix using the composition above.

## Rotations and quaternions

- Rotations are right-handed and active: they rotate vectors in the stated coordinate frame.
- Quaternion components, when serialized as scalar fields, are ordered `x, y, z, w`.
- The identity rotation is `(0, 0, 0, 1)`.
- Normalize quaternions at input boundaries when data may be externally authored or numerically accumulated.
- Interpolation must account for quaternion sign equivalence; use shortest-path spherical interpolation for general rotation blending.

## Camera and projection

- A camera's local right/up/forward axes are +X/+Y/+Z.
- The camera looks along its local +Z axis in engine/world space.
- The view transform converts world coordinates into camera coordinates; in view space the visible direction is -Z, matching the conventional right-handed view space used by graphics APIs.
- Projection is right-handed with a depth range of [0, 1], matching the depth convention expected by wgpu.
- Perspective field of view is vertical and expressed in radians.
- Near and far clip distances must be positive, with far greater than near. Infinite-far projection is not part of the Phase 0 contract.

This deliberately separates the engine's world-forward direction from the camera-space direction used by the projection matrix.

## GLTF import boundary

- GLTF data is right-handed and uses +Y as up.
- GLTF camera local viewing direction is -Z. This does not redefine ZYNE's +Z world-forward convention.
- Importers must preserve authored node and mesh transforms and must not apply a blanket 180-degree rotation to every asset.
- Any basis conversion required for a specific imported asset or animation must happen once at the importer boundary and be covered by fixtures/tests.
- Imported camera orientation must be converted deliberately so that its effective viewing direction agrees with ZYNE's +Z engine camera-forward convention.
- GPU clip-space conversion belongs in the renderer/projection boundary, not in scene or gameplay transforms.

## Implementation and testing rules

- The conventions above are API contracts even before concrete vector/matrix types are introduced.
- When concrete types are added, tests must cover vector basis directions, positive axis rotations, transform composition order, parent-child composition, camera view direction, [0, 1] projection depth, and GLTF conversion fixtures.
- Do not duplicate coordinate conversion in gameplay, animation, renderer, and asset consumers.
