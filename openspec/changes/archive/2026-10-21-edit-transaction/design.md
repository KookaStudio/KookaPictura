# Design

## Context

See `proposal.md` for motivation and `specs/document/edit-transaction/spec.md` for the contract. The current state that shapes the approach:

- The app bridge is `qobject::PictureView` (a cxx-qt QObject wrapping `PictureViewRust`). Command code lives in free functions with their own bridges as well as methods on the QObject; all of them can call `Pin<&mut PictureView>` methods.
- `PictureViewRust` owns `doc: Option<Document>`, `selection`, `history`, `dirty`, `content_revision`, `link_sets`, the level-0 frame, the view pyramid, and damage.
- `recomposite` = `current_buffer` (one backend choice) → `store_composite` → `reset_pyramid` → `changed`. `refresh_region`/`refresh_regions` = region composite → patch `doc.composite` (except during a mask stroke, which presents without patching the pre-stroke composite) → patch level-0/pyramid/damage → one `region_blitted` and deliberately no `changed`.
- `record` captures one history state from the current document and selection, sets `dirty`, and bumps `content_revision`; `record_move` omits the revision bump.
- Nine engine call sites share a crate-private `recompute` at the end of the geometry operations; the app then recomposites, so those commands composite twice.
- `PictureView` has no Rust constructor under cxx-qt 0.10; Rust tests exercise `PictureViewRust` helpers, and QObject behaviour is tested through Qt Test and the C++ self-test.
- `PictureView::commit_move(dx, dy)` already exists (`impl_transform/session.rs:781`); the new transaction entry point needs a different name.

## Goals / Non-Goals

**Goals:**

- One commit pipeline with a declared outcome, a fixed order, and one place that knows the region-vs-full, link, revision, and history rules.
- Migrate every hand-written commit tail, including multi-rectangle paint commits and record-only preview commits.
- Remove the engine's composite side effect so each command composites once, without losing merged-only documents.
- Keep transaction semantics unit-testable in Rust without a QObject.

**Non-Goals:**

- Preview coalescing (#448) and per-dab mask painting (#404): the transaction covers commits, not live previews (it provides the `preview` entry point they already need).
- Any C++/Qt signal or panel change. The region path keeps its no-`changed` contract.
- A document-session abstraction (#740); this transaction is the seam that session will use.
- History storage, plane sharing, or damage-account redesign.
- Removing redundant history states from commands that record at a repeated value; that is a separate behavior audit.

## Decisions

### A declared outcome, not inferred damage

```rust
enum Edit {
    None,                   // no-op
    Meta,                   // metadata/selection: record + changed
    Record,                 // a preview already presented it: record only
    Regions { rects: Vec<PsdRect>, notify: bool }, // bounded pixel change: one blit, record
    Full,                   // unbounded pixel change: composite, record, changed
    Structural,             // invalidates positional links: clear links, composite, record, changed
}
```

The mutation returns the outcome; the transaction returns the same outcome to the caller. Alternatives rejected:

- **Compare the document before/after** to detect change and derive damage. Correct but expensive (deep compare, including planes) and still cannot know that a selection-only change needs no composite.
- **Keep a per-command bool + rect** and let the transaction infer. That is the current duplication with a new name.

`Regions` carries a vector because a paint commit's dirty area is a set of disjoint tiles (`refresh_regions`); a single rectangle would force the bounding box and lose the tiled commit (#404’s area contract). Its `notify` flag preserves the two existing bounded-commit behaviours: property commits (`batch_property`) emit `changed`, while visibility toggles, paint commits, and previews do not.

`Structural` is defined by **link invalidation**, not by tree-only mutation. Besides layer-tree edits, the current code drops link sets for Pictura Raw (`impl_pictura_raw.rs:80`), type conversion and type styles (`type_tools.rs:478,531,733,750,845,926`), rasterizing a shape (`shapes.rs:611`), and free transform (`impl_transform/session.rs:535,541`). Those commands keep their current link behavior by declaring `Structural`; a filter declares `Full` and keeps links.

`Record` exists because `adjustment_commit` and `layer_style_commit` currently capture history without emitting `changed` (`adjustment_edit.rs:192-194`, `layer_style.rs:208-210`). A single `Meta` that always notifies would add signal counts; `Record` preserves the current behavior.

### The mutation gets `&mut PictureViewRust`

Most commands need only the document, but about ten need the selection, stroke state, active layer, GPU flag, or replace `doc` wholesale (fills, crop, type edit, filters). A narrower context type would grow accessors and replace-doc plumbing for no measurable safety, so the closure receives the backing state and an explicit rule lists what the transaction owns (history, revision, dirty, link sets, damage, signals).

### One order: mutate → composite → capture → notify

```rust
impl qobject::PictureView {
    fn edit(
        mut self: Pin<&mut Self>,
        label: &str,
        f: impl FnOnce(&mut PictureViewRust) -> Edit,
    ) -> Edit;
}
```

The implementation is:

1. Run `f(&mut rust)` and keep the returned `Edit`.
2. If `Edit::None`, return it without touching the document.
3. Execute the pixel half selected by that `Edit`: `Regions` → `refresh_regions`; `Full`/`Structural` → `composite_full` (a no-signal split of `recomposite`); `Meta`/`Record` → nothing.
4. Clear the link sets for `Structural` (link sets are not part of history, so before or after capture is equivalent; clearing before keeps the invariant obvious).
5. Call `PictureViewRust::commit(label, edit)` — capture one history state, set `dirty`, bump `content_revision` (unless the move entry point says otherwise).
6. Notify: `Meta`/`Full`/`Structural` emit `changed`; `Record` never does; `Regions` emits only when its `notify` flag is set (`refresh_regions` already emitted the region blit that drives the shell fan-out).

This makes “refresh before capture” structural: `commit` is only reachable after step 3.

`recomposite` stays as `composite_full` + `changed` for undo, redo, history jumps, and open; `edit` uses the split so the single `changed` fires after capture. The C++ `refresh()` keys on `frame_revision`, so a region commit that does not emit `changed` is unchanged from today.

### Split a Rust-testable bookkeeping core from the QObject execution

```rust
// PictureViewRust — bookkeeping only, unit-testable
fn commit(&mut self, label: &str, edit: Edit);
```

`commit` captures history, sets dirty, and bumps the revision. It does not composite and does not emit. A separate pure helper maps an `Edit` to its presentation:

```rust
enum Effect { Nothing, Notify, Blit, Rebuild }

fn effect_of(edit: &Edit) -> Effect;
```

The QObject `edit` executes the effect and emits. Rust tests assert `commit` (no-op, one state, one revision bump, link clearing) and `effect_of` (Regions → Blit, Full/Structural → Rebuild, Meta → Notify, Record → Nothing) without a QObject. Alternative rejected: test everything through Qt Test, which contradicts the issue’s explicit Rust test surface and slows the feedback loop.

The return value of `edit` is the declared `Edit`, not `Effect`, so a command can report the exact outcome.

### Previews and the move exception stay explicit

- `preview(edit)` uses the same presentation dispatch, captures no history, and bumps the revision. It is for adjustment, opacity, fill, mask-refine, shape-path, and other live edits whose composite must stay current.
- `commit_move_edit(label, edit)` captures one state without bumping `content_revision`, preserving the move-preview cache key. The name avoids the existing `commit_move(dx, dy)`. The move paths that need it are `translate_layer` (`impl_transform/dispatch.rs:144-173`), `commit_move` (`impl_transform/session.rs:781-818`), and `commit_move_legacy` (`impl_transform/session.rs:822-827`).
- Commit-only callers that follow a preview use `Edit::Record`: `adjustment_commit`, `layer_style_commit`, and the path commit.
- `edit_target` and `edit_adjustment` become preview mutations: they return the changed state and call `preview`; their commit callers (`paths::commit`, `adjustment_commit`) own the history entry. This preserves their current “refresh now, record later” behavior instead of recording twice.

### Preserve the existing signal policy

`Meta`, `Full`, and `Structural` notify; `Record` never does; `Regions` notifies only when its `notify` flag is set (property commits set it; visibility toggles, paint commits, and previews do not). The region contract is already load-bearing: `frame.cpp:452-478` blits the region, starts the panel timer, refreshes titles and the registry, and selftest 363 (`visibility toggle: blits >= 1, changed == 0`) asserts no `changed`. The known consequence is pre-existing: the History panel only listens to `changed`, so a bounded commit is not reflected there until the next full change. Fixing that is a separate C++/signal change.

Alternative considered: emit one `changed` after every commit and update selftest 363 to assert `frame_revision` instead. It would fix the History panel, but it changes observable signal counts and a milestone oracle for a refactor whose contract is “no behaviour change”. Rejected for this change.

### Engine operations stop recomputing, and merged-only documents materialize first

Delete `document_ops::recompute` and its nine call sites: `resize.rs:48`, `canvas.rs:42` (`resize_canvas_document`), `canvas.rs:94` (`extend_background`), `crop.rs:64` (`crop_document`), `crop.rs:126` (`delete_cropped_pixels`), `crop.rs:183` (`translate_layer`), `orient.rs:163`, `rotate_in.rs:379`, and `layer_ops/perspective_crop.rs:135`. The ops already return with the document structure final; the app’s single refresh stores the composite. Tests that asserted `doc.composite == composite_rgba(doc)` explicitly assign the composite before asserting or writing.

`translate_layer_active` (`crop.rs:210-226`) keeps its `composite_active` call: it is the CPU/GPU oracle (`ui/document-canvas`, “Layer translation uses the active backend”) and has no production caller. `translate_layer_path`, used by the app, never recomputed.

A merged-only document is the edge case that would otherwise regress: `resize_document`/crop/orient/canvas change `doc.width/height` while `doc.composite` keeps the old size; `current_buffer` returns that composite when `layers` is empty (`helpers_composite.rs:205-211`), `store_composite` refuses a size mismatch (`helpers_composite.rs:56-59`), and `write_psd` rejects it (`write.rs:1030-1036`). The existing `background_from_composite` (`mode/planes.rs:100-130`) drops alpha and is intended for flattening, so add a dedicated `materialize_merged_composite(doc)` that:

- runs only when `doc.layers.is_empty()` and the composite’s dimensions match the document;
- builds channels `0..color_channels` from the composite’s colour planes and channel `-1` from its alpha plane (or full opacity when absent);
- installs a bottom `Background` layer covering the document.

Each geometry operation calls it before mutating. The app then composites once; display and save see the transformed pixels. This intentionally changes the structure of a merged-only document on its first geometry edit (it gains a Background), matching Photoshop’s opened-flattened-file model. Other edits on merged-only documents keep their current behavior.

Mode, bitmap, indexed, and HDR conversions keep writing `doc.composite`: their composite is the converted representation, not a cache. An 8/16-bit depth change keeps the existing 8-bit display composite (`mode/mod.rs:136-157`).

### Migration shape

1. Add the `Edit`/`Effect` contract, stubs, and the tests named in `test-plan.md`.
2. Implement `commit`, `edit`, `preview`, `commit_move_edit`, and `composite_full`; make the core tests pass.
3. Migrate the ten helpers and the explicit audit list: `set_layers_visible`, `commit_layers_opacity/fill`, `delete_layers` (Structural), `apply_visibility`, metadata/profile commands, `apply_selection_labeled`, `paths::commit`, `path_list::commit`, `guides::commit`, `annotations::commit`, `layer_style_commit`, `adjustment_commit`, image match/replace/HDR, filter jobs, paint and mask commits (`Regions`). Add a grep guard for `record(`, `record_move(`, `refresh_region(s)(`, and `recomposite(` under the app with an allowlist for undo/open/cancel/preview.
4. Remove engine `recompute`, add `materialize_merged_composite`, update the nine call sites, the geometry specs, and the engine/unit/oracle tests.
5. Convert adjustments, paths, layer styles, and resolution-only Image Size to the preview/record split.

## Risks / Trade-offs

- **Misdeclared outcome** (a pixel change declared `Meta`, or a link-invalidating edit declared `Full`) leaves a stale display or stale links → keep `layer_visibility_region` and `changed_property_region` as derivation helpers; review the audit list by outcome; region tests and selftest 363/541 catch stale pixels.
- **Stale composite in history** → the mutate → composite → capture order is structural; a Rust test asserts a commit after a refresh captures the refreshed composite; undo byte-identity tests guard the display.
- **Merged-only materialization changes the document structure** → only on the first geometry edit, only for a no-layer document with a matching composite; tests cover display, save, and undo.
- **Signal-count regressions** → `Record` exists for the record-only commits; selftest 363 and `tst_layers_preview_coalescing` are the guards.
- **Engine test/oracle churn** → geometry tests and `document_oracle.rs` explicitly materialize `doc.composite = composite_rgba(&doc)` before writing or asserting; the oracle also asserts the engine left it untouched.
- **File size** → the transaction lives in a new module; `impl_layers.rs` (1055 lines) is rewritten in place and checked per phase, not only at the end.
- **A one-off exception leaks into the interface** (`commit_move_edit`) → it is one named entry point shared by the three move paths, and the doc comment states why.

## Migration Plan

Land as one change on `refactor/739-edit-transaction`, phased by commit so each step is independently reviewable: contract + tests, transaction core, helper/audit migration, engine recompute removal + materialization, preview/record conversion, verification. The three main specs are modified in the same change; archiving merges them. No data migration or compatibility shim is needed; history files and the PSD format are untouched. Rollback is reverting the commits: the transaction adds no persisted state.
