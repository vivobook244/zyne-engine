# Resource Identity and Lifecycle

This document defines the Phase 0 resource contract. It does not implement an asset database, importer, cache, or GPU resource manager.

## Two different identifiers

### Persistent identity: `ResourceId`

- `ResourceId` wraps a UUID v4.
- Create it once when a resource is first registered/imported, then persist it in project metadata.
- Reopening a project or loading a scene must preserve the same ID.
- Copying or importing an asset as a new independent resource must create a new ID.
- Renaming or moving an asset must not change its ID.
- A resource ID identifies the logical asset, not a particular in-memory object or GPU allocation.

### Runtime reference: `Handle<T>`

- `Handle<T>` is a typed, process-local reference made from a slot index and generation.
- The type parameter prevents accidentally passing (for example) a texture handle where a mesh handle is expected.
- A registry validates both index and generation before returning an object.
- Releasing a slot increments its generation before reuse. An old handle then becomes stale and must fail lookup.
- A handle is not persistent, must not be serialized, and must never be used as a scene or project identifier.
- Handles may be copied; copying does not extend resource lifetime or guarantee validity.

## Ownership and lifecycle

The future resource manager owns loaded runtime objects. Systems may borrow them through validated handles; consumers do not independently free manager-owned objects.

The intended lifecycle is:

1. **Unloaded** — persistent metadata exists, but no runtime object is resident.
2. **Loading** — the manager is resolving source data and constructing the runtime representation.
3. **Loaded** — the runtime object is available through a valid typed handle.
4. **Failed** — loading failed; diagnostic information is retained and the manager may retry.

Unloading invalidates all handles to the released slot. A later reload may receive a different slot or generation. Code must reacquire a handle after unload/reload rather than assume the previous one remains valid.

Phase 0 does not prescribe synchronous versus asynchronous loading, eviction policy, reference counting, or dependency scheduling. Those belong to the asset pipeline design.

## Metadata versus runtime objects

Persist resource metadata, not runtime allocations.

Persisted metadata is expected to contain:
- `ResourceId`;
- resource kind/type;
- source URI or project-relative source path;
- importer settings and source/dependency metadata as required by the asset type.

Do not serialize:
- `Handle<T>`;
- pointers, references, or process-local slot coordinates;
- GPU buffers, textures, bind groups, or other device-owned objects;
- transient caches or backend-specific allocation state.

A scene stores `ResourceId` references. During scene loading, the resource manager resolves those IDs and supplies fresh runtime handles. Missing IDs or failed loads must produce explicit errors or a documented fallback; they must not silently resolve to an unrelated resource.

## Current implementation boundary

`zyne-core` provides `ResourceId` and `Handle<T>` as lightweight contracts. It intentionally does not depend on an asset format, renderer, GPU API, filesystem, or resource registry implementation.
