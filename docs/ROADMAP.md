# ZYNE Development Roadmap

Each phase must produce a verifiable result before the next phase expands the architecture.

| Phase | Focus | Estimate | Acceptance |
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
