# ZYNE Development Roadmap

**Last reconciled:** 2026-11-24  
**Repository:** `vivobook244/zyne-engine`  
**Active branch:** `main`  
**Last verified HEAD before this documentation update:** `0b784cb6335728e82f8138048ea9e01bfa3a0f7e`

This is the master roadmap and status index for Phases 0–11. Whenever a phase or subphase is passed, record the transition in its tracker or the latest active-work Markdown file, including date, commit, validation evidence, and remaining limitations.

## Current work

**Active milestone: Phase 5.5.4 — Material Texture Integration.**

Implemented: CPU RGBA8 texture data and validation, 8-bit glTF image normalization, runtime diffuse texture resolution, GPU texture upload/cache/draw binding, showcase integration, and runtime cache lifecycle reconciliation.

Physical GPU showcase validation was performed on 2026-11-24 using Intel UHD Graphics 610 (Vulkan). Blue ceramic and amber diffuse textures were visible, the checker fallback remained visible, and viewport resizing followed the window without breaking the textures. Focused automated coverage that instantiates actual GPU allocations and exercises cache reuse, fallback selection, and per-draw binding remains open. See [Phase 5.5.4 tracker](development/PHASE-5-5-4-MATERIAL-TEXTURE-INTEGRATION.md).

## Phase status index

| Phase | Focus | Status | Completion / transition record |
|---|---|---|---|
| 0 | Project Foundation | COMPLETED | Foundation completion record; see Phase 0 development documentation |
| 1 | Platform & Application Core | IMPLEMENTED; formal closure audit required | Phase 1.5 lifecycle tracker still contains pending acceptance/validation notes |
| 2 | Rendering Foundation | FOUNDATION COMPLETE; audit record required | Phase 2.6 texture/camera tracker and local GPU validation |
| 3 | World, ECS & Scene | COMPLETED | [Phase 3 tracker](development/PHASE-3-WORLD-ECS-SCENE.md) |
| 4 | Asset & Resource Pipeline | IMPLEMENTATION ADVANCED; formal closure audit required | [Phase 4 tracker](development/PHASE-4-ASSETS-RESOURCES.md) has stale open items |
| 5 | Materials, Lighting & Environment | IN PROGRESS | [Phase 5.5.4 tracker](development/PHASE-5-5-4-MATERIAL-TEXTURE-INTEGRATION.md) |
| 6 | Physics & Collision | PLANNED | Not started |
| 7 | Character & Animation | PLANNED | Not started |
| 8 | Camera & Gameplay Framework | PLANNED | Not started |
| 9 | Audio & Visual Effects | PLANNED | Not started |
| 10 | ZYNE Studio | PLANNED | Not started |
| 11 | Build, Optimization & Release | PLANNED | Not started |

Phases 1, 2, and 4 are not marked fully closed until their older trackers and current source/tests have been reconciled. Status labels distinguish implementation from verified completion.

## Phase sequence and acceptance

| Phase | Focus | Estimate | High-level acceptance |
|---|---|---:|---|
| 0 | Project foundation | 3–5 days | Workspace checks; architecture and conventions documented |
| 1 | Platform & application core | 1 week | Stable Windows window, input, resize, timing, shutdown |
| 2 | Rendering foundation | 2–3 weeks | Textured 3D model with controllable camera |
| 3 | World, ECS & scene | 1–2 weeks | Multi-entity scene saves/reloads with transforms |
| 4 | Asset & resource pipeline | 1–2 weeks | Textured GLB imports with stable references |
| 5 | Materials, lighting & environment | 2–3 weeks | Multiple materials/lights and directional shadows |
| 6 | Physics & collision | 1–2 weeks | Collision and trigger detection work |
| 7 | Character & animation | 2–3 weeks | Idle/walk/run/jump/fall transitions work |
| 8 | Camera & gameplay framework | 2 weeks | Third-person character, aim mode, interactions |
| 9 | Audio & visual effects | 1–2 weeks | Audio and basic effects without renderer instability |
| 10 | ZYNE Studio | 4–6 weeks | Create, edit, save, and run a simple level |
| 11 | Build, optimization & release | 3–5 weeks | Sample project builds/runs without Studio |

Estimates are planning ranges, not guarantees. A tightly scoped first release is roughly 5–8 months depending on integration and debugging.

## Phase implementation sequence

- **Phase 1:** 1.1 Architecture Review & Decisions; 1.2 Window Management; 1.3 Event System; 1.4 Input Abstraction; 1.5 Application Lifecycle.
- **Phase 2:** 2.1 GPU Initialization; 2.2 Surface Resize/Suspension; 2.3 Frame Acquisition/Presentation; 2.4 Shader Pipeline; 2.5 Depth-tested 3D Foundation; 2.6 Textures & Controllable Camera. See [Phase 2.6](development/PHASE-2-6-TEXTURES-CAMERA-CONTROL.md).
- **Phase 3:** See [Phase 3 — World, ECS & Scene](development/PHASE-3-WORLD-ECS-SCENE.md).
- **Phase 4:** See [Phase 4 — Assets & Resources](development/PHASE-4-ASSETS-RESOURCES.md).
- **Phase 5:** Material and lighting foundation; 5.5.1 HDR; 5.5.2 Tone Mapping; 5.5.3 Exposure Control; 5.5.4 Material Texture Integration. Detailed status belongs in the relevant Phase 5 trackers.

## Milestones

- M1 First 3D Render: window, mesh, texture, camera.
- M2 3D World: scene, ECS, transforms, assets.
- M3 Visual Quality: materials, lighting, shadows, fog, environment.
- M4 Playable Character: physics, skeleton, animation.
- M5 Gameplay Runtime: interaction, audio, playable environment.
- M6 ZYNE Studio: essential scene editing workflow.
- M7 ZYNE 0.1: standalone build, packaging, documentation.

## Initial scope

Windows first, Linux second; 60 FPS target; 1280×720 baseline and output up to 1080p, hardware dependent.

Out of initial scope: network multiplayer, ray tracing, PBR-first rendering, visual scripting, node shader editor, advanced terrain, plugin marketplace, and console platforms.

## Development recordkeeping rule

Whenever a phase or subphase is passed, update its tracker or the latest active-work Markdown file in the same work sequence. Record:
1. Date and exact commit.
2. Scope completed and explicit non-goals.
3. Automated test results.
4. CI platform results.
5. Physical GPU/runtime validation when relevant.
6. Acceptance criteria checked and remaining unchecked items.
7. Next milestone and its prerequisites.

Never mark a phase COMPLETED while required acceptance items remain unchecked. Keep this roadmap as the index; detailed implementation evidence belongs in phase trackers.
