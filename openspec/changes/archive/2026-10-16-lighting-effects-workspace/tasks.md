# Tasks: lighting-effects-workspace

## 1. Engine

- [x] 1.1 Move Lighting Effects to `render/lighting.rs`; `Lighting` becomes the rig (shared properties + `Vec<Light>`, 1..=16).
- [x] 1.2 CS6 geometry: elliptical Spot with far-end hotspot, Point radius, Infinite angle + elevation; intensity ≈ 50 normal.
- [x] 1.3 Port photorust's Lighting tests to the rig and add hotspot, rig-sum, hidden-light, colour, and rejection tests.

## 2. App

- [x] 2.1 Variable-arity `lighting-effects` mapping (9 + 13·k slots) with tests; update the spec row and the whole-layer preview test.
- [x] 2.2 `lighting_rig` (slots, names, CS6 presets), `LightingCanvas` (on-canvas controls), and `LightingEffectsDialog` (options bar, Properties, Lights).
- [x] 2.3 Route the menu and Last Filter Settings to the workspace.

## 3. Verification

- [x] 3.1 Add the `tst_lighting_effects_dialog` Qt Test suite; update `tst_filter_menu`'s slot count.
- [x] 3.2 Update `docs/06-filters/lighting-effects.md` and `render-filters.md` (`TASK-ALLOWS-DOCS`).
- [x] 3.3 Run `bash scripts/verify-full.sh` and `openspec validate --all --strict`.
