# Proposal

## Why

Issue #1349 identifies two contradictory active contracts: `compositing/view-pyramid` requires whole stored reduced levels, while the production `TilePyramid` contract forbids retaining them. The unused production `ViewPyramid` API still compiles into ordinary builds even though its remaining consumers use it as an independent test oracle.

## What Changes

- Retire all five stored-level requirements in `compositing/view-pyramid`; preserve their still-relevant pixel semantics and level topology in the region and tile capabilities.
- **BREAKING**: remove `Planes` and `ViewPyramid` from ordinary renderer builds and exports. Expose the minimal oracle only for renderer unit tests or the non-default `test-oracle` feature; enable that feature through the app's renderer dev-dependency.
- Delete `coarse`, `coarse_level`, `fits`, `update`, and `patch_level`, their obsolete helpers, and their method-specific tests. Retain independent `rebuild`, `level_count`, `level_size`, and `crop` coverage.
- Keep `PyramidLevel`, `SMALLEST_SIDE`, and `TILE` available unconditionally, without sharing the oracle's level or pixel arithmetic with production.
- Replace active stored-level assumptions with logical-level and tile-cache wording, preserving Navigator, mask solo, previews, crop reuse, and exact committed pixels.
- Keep the existing `rc_levels`, `tp_crop`, `tp_damage`, and `tp_solo_restore` comparisons unchanged; add checks for the test-only build boundary.

## Capabilities

### New Capabilities

- `verification/view-pyramid-oracle`: independent whole-rebuild reference, minimal surface, and exclusion from ordinary release/dev-app builds.

### Modified Capabilities

- `compositing/view-pyramid`: remove the five obsolete stored-level requirements.
- `compositing/region-composite`: specify exact premultiplication, rounded halving and odd-edge folding without requiring stored levels.
- `compositing/tile-pyramid`: own logical level count, dimensions and stopping rules; reference region pixel semantics.
- `compositing/render-session`: express level availability and preview fallback without requiring stored levels.
- `ui/document-canvas`: replace borrowed whole-level presentation with cached tile crops while preserving zoom selection and crop reuse.
- `ui/navigator-panel`: identify the tile pyramid as the provider of reduced thumbnail pixels.
- `ui/mask-solo-view`: describe independent mask/composite tile sources instead of a stored composite pyramid.
- `ui/canvas-frame-reuse`: identify tile-pyramid crops as the canvas source.
- `ui/info-histogram-panel`: clarify the coarsest tile-level source and the bound on histogram binning.
- `tools/paint-engine`: express reduced previews and frame-bounded display repair in tile terms.
- `tools/mask-stroke-preview`: identify the reduced tile level used for a mask preview.
- `verification/verification-harness`: replace obsolete whole-pyramid rebuild observations with frame-bounded tile refresh checks.

## Impact

Renderer module boundaries, exports, `crates/pictura-render/Cargo.toml`, `crates/pictura-app/Cargo.toml`, and the oracle's unit tests change. Existing renderer and app oracle callers retain their imports and comparison logic. No new dependency, GUI behavior, rendering algorithm, file format, or golden baseline is introduced. The dependent specs are reconciled through deltas; main specs are merged only when the implemented change is archived. Historical archived proposals remain unchanged. No `docs/` edits are planned.
