# Design

## Context

See `proposal.md` for motivation and
`specs/document/coverage-view/spec.md` for the engine interface contract.

The existing selection is a document-sized `Plane<u8>` in `pictura-select`.
Core owns both `Plane` and `LayerMask`, while render and paint depend on core.
The app's `helpers_composite.rs` wraps a selection in a `LayerMask` for filter,
adjustment, matting, copy/cut, and other consumers. Its `selection_mask_for`
also copies the selection into a mask-local crop. Some calls instead attach
the result as a real mask on a new fill or adjustment layer.

Render's `filter::coverage` and `coverage_row` sample raw bytes plus exterior
color and disabled state. `mask_refine::resolve_raster`, listed in the issue's
selection-parameter inventory, actually resolves a real mask: density,
feather, disabled state, missing data, and cache identity matter there.
Compositing treats missing mask data as full coverage; the destructive-filter
sampling helper currently falls back to the exterior value. These policies
cannot be collapsed into an unconditional mask-to-selection conversion.

Paint already has tile-aware `Through` inputs for fill and a retained
`CoverageClip` for strokes. Fill's `uniform_over` branch skips empty tiles and
blends uniform tiles once. Clipboard uses document-sized planes but bounds
its output to the selected region. Patch and Content-Aware Move accept a
plane, then build their solver's bounded floating-point work region.

Existing tests provide reusable fixtures: render's
`selection_filters_read_a_tiled_selection`, paint's
`healing_reads_a_tiled_selection` and stroke clipping checks, and the app's
`selection_readers_do_not_materialize`. Their assertions remain regression
evidence; new checks target the new interface and additional scenarios.

## Goals / Non-Goals

**Goals:** keep ownership, coordinate translation, exterior sampling, and sparse
reads behind one small core interface; make the selection origin irrelevant to
consumers; migrate retained and immediate operations together.

**Non-Goals:** moving selection algorithms into core, replacing tiled storage,
changing healing solvers or feather kernels, changing native mask precision,
changing compositing policy, or adding GUI behavior. Existing bounded output
buffers and solver arrays remain legitimate outputs/workspace. This work does
not remove Refine Edge's separately documented dense-work-array exception.

## Decisions

### 1. One concrete, shareable coverage value in core

Add `crates/pictura-core/src/coverage.rs` and export `Coverage`. Store private
signed `PsdRect` bounds, shared `Plane<u8>` storage, and an exterior byte.
Support constant coverage without a dense plane, including implicit full
coverage. The precise private representation can use an optional plane; an
absent plane there means a constant value, never an absent editing constraint.

Take `&Coverage` or `Option<&Coverage>` at consumer interfaces. Retained strokes,
jobs, previews, and proxies own the same `Coverage` type. Its construction and
cloning share `Plane` storage and never call `as_slice`, `Deref`, `to_vec`, or
build a full-frame mirror. Use scoped borrows of the stored plane for reads;
do not expose a slice-based alternative selection interface.

This avoids a trait hierarchy and a second owned-versus-borrowed coverage
type. A borrowed-only view would force retained operations to invent wrappers
or separately carry geometry; generic callbacks would hide tile uniformity.
`TiledMask` remains an internal plane alias for mask-producing kernels.

### 2. Put geometry and sparse access together

The interface provides checked construction, bounds, point sampling, a row
read into caller storage, a uniform-region query, covered bounds within a
finite operation region, and rebasing to a local origin. Keep the storage
private so consumers cannot silently regain canvas-width indexing.

Construction validates plane length and tiled geometry against the half-open
rectangle using widened, checked arithmetic. Zero-area and constant coverage
are valid. Reject inconsistent dimensions rather than reshaping tiled data.
Rebasing computes translated bounds with widened arithmetic and refuses an
unrepresentable result. It shares storage and changes neither the source
view nor its plane identity. A mask-local edit uses a view translated by the
negative mask origin, preserving all exterior samples without cropping.

Row reads fill the output with the exterior value, then copy the intersecting
span through the existing plane accessors. Point reads check both coordinates
before calculating an index, preventing an out-of-range x from wrapping into
the next row. Uniform and covered-bounds queries operate within the requested
finite region and account for exterior values. Constant, empty, and uniform
tiles stay cheap; mixed tiles are read only as required. No algorithm scans
the whole document just to adapt a small selection.

### 3. Producers preserve domain policy

- `Selection` exposes coverage from its own width, height, and plane with a
  zero exterior. Do not infer selection dimensions from a target layer.
- Alpha channels use explicit document dimensions and existing size checks;
  obtaining a view does not export and re-import a byte vector.
- Quick Mask projects its canonical selected coverage. Its selected-areas
  display inversion remains presentation/edit interpretation, and its
  implicit-full state projects to constant 255. The independent temporary
  selection remains the operation's optional constraint.
- A raw raster-mask projection exposes bounds, stored bytes, and exterior
  default only. It does not encode density, feather, link flags, or enabled
  state. Mask-aware callers preserve their existing disabled, missing-data,
  malformed-data, and native-depth decisions before using raw coverage.

In particular, keep `resolve_raster` accepting a real `LayerMask`; share raw
sampling underneath its plain/feathered processing where appropriate. Keep
actual feather and density application, native samples, cache keys, and
missing-data-as-full behavior in the renderer. The old destructive helper's
disabled and missing-data policies remain explicit at real-mask call sites;
they do not become properties of selection coverage.

### 4. Migrate consumers and retained snapshots atomically

Renderer migration includes `filter.rs`, `apply_image.rs`, `filter_proxy.rs`,
selection-driven mask creation, `document_ops/matting.rs`, Layer via Copy/Cut,
selected-content moves, clipboard, and any selection-gated smart-filter path
reached by adapter removal. A proxy reduces selection coverage separately
from actual layer masks; it must not call a layer-mask reduction helper that
also carries metadata. Preserve special operation policies such as Liquify's
existing selection-box handling and transparency locks.

Paint migration covers `Through` selection inputs and source coverage where
they cross that seam, gradient, bucket/flood selection walls, eraser, stroke
clips and per-dab restoration, Patch, and Content-Aware Move. Retain tile
uniform decisions in `fill_uniform.rs` through coverage queries. Replace the
stroke clip wrapper with retained `Coverage`, preserving document coordinates
as layers grow. Internal tip masks, flood outputs, solver arrays, and pixel
images remain their existing data structures; only editing coverage crosses
the shared seam.

Clipboard's public selection inputs move from raw planes to coverage. Reuse
covered bounds and row reads, keep `Clip`'s bounded RGBA/mask output format,
and preserve Into/Outside placement and persistent-mask construction. Healing
reuses coverage bounds/rows to populate the same bounded solver arrays in the
same sample order. A named raw-plane compatibility entry is not retained.

App migration includes filters and image-adjust jobs, layer creation and
matting, selection moves/refinement outputs, filter tools/proxies/depth
sources, Quick Mask, clipboard, and paint/healing callers. Capture one shared
coverage value where an edit must outlive a borrow; do not replace each old
adapter with another per-operation geometry wrapper.

### 5. Persistent mask creation is explicit

Separate consuming coverage from attaching a mask. Fill/adjustment defaults,
Add Layer Mask, Refine Edge mask outputs, and masked paste construct actual
`LayerMask` values only at their existing attachment points. Preserve each
operation's current bounds, exterior, flags, default settings, native-plane
invalidation, and refusal behavior. Do not normalize different existing mask
creation policies as part of this refactor.

Remove both app helpers and every production call, including calls that now
construct persistent masks. This is an inventory obligation, not a frozen
count of 20 call sites: current source also includes required `&LayerMask`
selection inputs in via/move code beyond the issue's optional-parameter list.
Retain real `LayerMask` interfaces that actually consume metadata.

### 6. Verify through the public seam

`test-plan.md` maps every delta scenario to a named std Rust check. New core
tests exercise direct sampling, signed origins, initialized rows, mismatched
geometry, translation overflow, constant/uniform regions, and tile sharing.
Cross-crate checks pass the same coverage type to real consumers; existing
pixel, rounding, metadata, refusal, and history assertions remain unchanged.

Use a generated rectangular selection large enough to retain empty and
uniform interior tiles after a real feather operation, with ramps crossing
256-pixel tile boundaries. Assert the fixture has partial bytes and all
three relevant tile classes before comparing filter and paint results with
the same coverage bytes held flat. Create the flat reference before starting
the materialization measurement. Use the repository's existing isolated
materialization-guard pattern to avoid process-wide counter interference.

The test plan names future tests honestly. During proposal-only work, the
repository's test-plan guard will report those absent functions until apply
creates them. Do not mark them manual, weaken the guard, or create test code
during planning. No manual parity checklist is needed for this engine refactor.

## Risks / Trade-offs

- Exterior/missing-data policies becoming one implicit default -> retain
  named raw projection and explicit real-mask resolution; test disabled and
  missing masks separately from absent/empty selections.
- Losing sparse fast paths behind generic sampling -> expose uniform-region
  and bounded coverage queries, retain tile-class checks and materialization
  guards, and keep small clipboard output bounded.
- Retained views copying coverage -> own the existing shared plane; test
  identity/sharing and unchanged source geometry when rebasing.
- Broad Rust interface migration -> finish all code and test edits before the
  first build; compile the affected crates and app together, then run targeted
  regressions and repository gates.
- Metadata accidentally standardized during real-mask creation -> compare
  complete masks and undo snapshots, including retained native/off-canvas data.

## Migration Plan

Finalization update (2026-11-17): the user requested archival and a PR after
the earlier CMake build, with all further builds handled by CI. The active
test-plan check and strict OpenSpec validation passed locally; runtime
regressions and the remaining gates are explicitly pending in CI. This
instruction supersedes the local-before-archive ordering in steps 3 and 4.

1. Add all test-plan checks and core coverage/projection support, then migrate
   render, paint, and app callers together. Adjust test fixtures at interface
   call sites without weakening their assertions.
2. Delete the synthetic adapters and use a production-source search to ensure
   neither survives. Review remaining `LayerMask` parameters for actual
   mask-specific responsibilities rather than blanket replacing them.
3. After all edits, build once within the 80% CPU budget, fix compile errors
   and rebuild only after code changes. Run coverage-specific and existing
   filter/matting/mask-refinement/clipboard/healing/stroke regressions and
   validate the test plan while the change is still active. No build runs
   during proposal creation.
4. Archive the completed change in the implementation PR after those checks
   succeed. Run the full repository gates, strict OpenSpec validation, and
   zero-open check after archival, then record the validation evidence in the
   PR. No feature is shipped/removed, no persistent format changes, and no
   data migration is required. Rollback is a coordinated source revert.
