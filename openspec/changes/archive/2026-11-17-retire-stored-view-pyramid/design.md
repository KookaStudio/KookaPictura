# Design

## Context

See `proposal.md` for motivation and issue #1349 for acceptance. Direct source inspection found the following boundaries; the graph index has no coverage for this worktree, so it was not used to establish caller completeness.

- `view_pyramid.rs` combines the shared `PyramidLevel` byte container and constants with the whole-rebuild oracle, borrowed `Planes`, two halving kernels, coarse sampling, damage repair, and patch tests.
- Production `tile_pyramid.rs`, `region_level.rs`, and `render_session.rs` import only the shared container/constants from that module. Their level and pixel arithmetic already differs structurally from the oracle and must stay independent.
- All app oracle consumers are in test modules: `canvas_view_test.rs`, `tiles_tests.rs`, `tests.rs`, the pan/zoom and restore profiles, and `open_jobs/imp/tests.rs`. The `planes_of` adapter in `helpers_composite.rs` is already test-gated.
- `pictura-render` has no default oracle feature; the workspace uses Cargo resolver 2. The app has an ordinary renderer dependency and can add a matching dev-dependency. `pictura-testkit` currently has no dependencies.
- The production display is `DisplayTiles` over `TilePyramid`. Navigator and Histogram consume composite tile-level crops (level 0 when no reduced level exists); mask solo changes the tile source, not the document composite. Some active specs still describe the pre-tile implementation.

## Goals / Non-Goals

**Goals:** make the production/test boundary compile-time explicit; keep byte comparisons independent; preserve the current display algorithm and public production container/constants; make the active contracts agree with tile-backed presentation.

**Non-Goals:** change production pixel math, cache budgets, preview approximations, zoom thresholds, renderer scheduling, file formats, or golden baselines; move the oracle into another crate; reconcile unrelated historical composite-storage contracts; edit `docs/` or archived changes.

## Decisions

### 1. Split the shared data container from the oracle

Move `PyramidLevel`, its existing methods, `SMALLEST_SIDE`, and `TILE` without behavior changes to `crates/pictura-render/src/pyramid_level.rs`. Keep their existing root exports and update the three production imports. Leave `view_pyramid.rs` as the small whole-rebuild oracle. Change its direct reads of the now-sibling container's private dimensions to the existing `width()`/`height()` accessors; do not widen field visibility or change reduction arithmetic.

Declare the oracle module and its `Planes`/`ViewPyramid` root re-exports with `#[cfg(any(test, feature = "test-oracle"))]`. Gate the entire module rather than scattering attributes across types and kernels, so ordinary builds cannot accidentally retain the helpers. Keep the shared container free of level-count, premultiplication, averaging, or sampling helpers.

Alternative: leave the shared items in `view_pyramid.rs` and gate many individual items. Rejected because future helpers could silently escape the test-only boundary and the module name would still imply a production stored pyramid.

### 2. Enable the feature only for cross-crate tests

Declare `test-oracle = []` in the renderer and enable it only in the app's `pictura-render` dev-dependency. Renderer unit tests use their own test cfg and must pass when invoked alone without the feature. App tests and ignored profiles keep their existing imports. Ordinary `cargo build -p pictura_app --lib` release/dev-app invocations do not request dev targets or the oracle feature.

Alternative: a plain test cfg breaks app tests because the dependency's library does not inherit the app's test cfg. Relocating the oracle into testkit would require moving or adapting the renderer's container and core plane/rectangle interfaces and changing callers; that adds coupling without improving this issue's boundary.

The exclusion contract applies to ordinary builds, not an explicit user request for `--all-features` or `test-oracle`.

### 3. Keep only the exact reference surface

Retain `rebuild`, `level_count`, `level_size`, and `crop`, with the independent `half`, premultiplication, rounded averaging, row readers, and rebuild kernels they require. Remove `coarse`, `coarse_level`, `fits`, `update`, and `patch_level`; remove `COARSE_PIXELS`, point sampling, tile-alignment/damage expansion helpers and tests used only by those paths. Preserve `TILE` even if retiring the damage implementation leaves no production caller.

Keep dimension, small-image, odd-edge, premultiplied soft-edge, transparent-outside, and tiled-source rebuild/crop assertions. Some existing tests mix retained behavior with coarse/update coverage: split those assertions rather than deleting the useful source-reading checks. Do not change the independent expected values in the four issue-mandated comparisons.

### 4. Separate logical pyramid semantics from storage

`compositing/region-composite` owns pixel semantics: ceil-halving of dimensions; premultiply color by rounded `c*a/255` before the first average; average each available 2x2 block with integer rounding; fold odd edges using only present samples; repeat one level at a time rather than averaging a large block once; return transparent samples outside the level.

`compositing/tile-pyramid` owns available levels. Preserve the current stop test exactly: compute the next ceil-halved dimensions, then stop if the next long side is below 256 or either next dimension is at most 1. This includes the thin-image termination condition that the former prose omitted; it is a description of existing code, not a new rendering policy. Add explicit thin-image coverage.

Remove all five old stored-level requirements. Use REMOVED deltas with migration destinations and declare `retire_capabilities: true` in this change's metadata: archive must delete the emptied `compositing/view-pyramid` capability rather than leave an invalid zero-requirement spec. For the six replacement requirements whose obsolete scenario titles change (canvas present, preview availability, histogram sampling, stroke frame repair, GPU-stroke presentation and verification), explicitly remove the old requirement and add its named tile-based replacement; OpenSpec rejects silent scenario renames inside MODIFIED blocks. Reference a whole rebuild as a test reference only; do not require production to allocate it. Conceptual uses of "view pyramid" in purpose text can still mean logical levels, but modified normative clauses and scenarios name tile sources and crops explicitly.

The canvas's stored-level zero-copy scenario is replaced by reuse of an assembled reduced tile crop; the old tag remains traceable. Crop overlap reuse, exact display bytes and existing zoom thresholds remain required. Preview commits repair or invalidate affected tiles from the exact document, and cancelled previews restore current tiles. A level-0 preview remains available and exact even when no reduced level exists; an unavailable requested level falls back, and unreducible content causes fallback only for reduced-level previews. Histogram fetches the coarsest level before scaling it to at most 512×512 for binning; thin images may fetch level 0 and source work is not universally bounded. Other stale references in the listed capabilities are rewritten without changing unrelated requirements.

### 5. Prove the build boundary and retain the numerical oracles

Use existing tests for numerical behavior. Add a small std test for the source/manifest boundary in the app build-configuration tests, a renderer topology test for thin images, and any Qt assertions identified as missing by `test-plan.md`.

Add `scripts/check-view-oracle.py`, using only Python's standard library, to audit Cargo JSON compiler-artifact records from an already completed normal build. Its profile check must fail if the renderer feature list includes `test-oracle`; it must compile a temporary external consumer against that build's renderer artifact to confirm the shared exports remain usable and oracle imports fail specifically as unresolved imports. It must also reject missing/ambiguous renderer artifacts and unrelated compiler failures rather than reporting a false pass. Provide `--self-check` cases for report validation and subprocess-result classification. The checker does not initiate Cargo builds or run inside a unit test.

Run the proof for both release and dev-app after implementation, separately from test-target feature activation. Do not infer exclusion from missing binary symbols: the linker could strip an always-compiled API. Source-boundary assertions plus feature records and negative external compilation establish the intended boundary.

## Risks / Trade-offs

- Feature unification can enable the oracle during test builds. Mitigation: audit separate normal library builds, never `--all-targets` or `--all-features`, and inspect actual artifact feature lists.
- A shared halving helper would make comparisons tautological. Mitigation: share only the data container/constants; assert that oracle source does not invoke production region/level helpers and retain exact comparisons.
- Deleting mixed coarse/update tests could lose rebuild or tiled-source coverage. Mitigation: preserve those assertions in retained or split tests before deleting obsolete paths.
- Broad spec replacement could discard unrelated scenarios. Mitigation: copy complete modified requirement blocks, preserve existing tags and scenarios, and change only storage/provider wording and obsolete observations.
- Negative compilation could fail for a toolchain or linking reason. Mitigation: require the positive shared-type probe to succeed first, and inspect the negative diagnostic for the missing oracle names.
- The test-plan existence guard will report newly planned tests until apply creates them. Mitigation: mark planned coverage explicitly and put test-writing tasks before production edits; do not label automatable checks as manual.

## Migration Plan

1. Add the planned verification checks, then split and gate the renderer oracle and configure the app dev-dependency. Preserve all ordinary production root exports except the intentionally retired oracle surface.
2. Remove dead methods and adapt mixed oracle tests without changing the four required numerical comparisons.
3. Build only after all implementation and checks are written, within the repository CPU budget. Run focused checks and both normal-profile artifact audits.
4. Merge deltas by archiving this change in the implementation PR, validate the resulting main specs, then run the integrated gates including the zero-open guard. Update the relevant ROADMAP entry and leave no open change at shipping time. Retain historical archive files unchanged.

Rollback restores the module/export split, manifest edits and deltas together. There is no data migration or serialized state change.
