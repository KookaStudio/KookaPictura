# Proposal

## Why

Every app-bridge command writes the same commit sequence by hand: mutate the document, refresh the composite (region or full), clear link sets, capture history, bump the revision, emit a signal. That sequence exists in 139 `record(` sites, 130 `recomposite()` sites, and about ten look-alike local helpers, so history, damage, compositing, and signalling have no single owner. Two visible defects follow: the ordering rule "refresh before record" is only a comment, and document-size engine ops composite once inside the engine and again in the app (#739).

## What Changes

- Add one edit-transaction interface on the app bridge. It takes a history label and a mutation that declares its outcome; the transaction owns compositing, linkage invalidation, history capture, revision and dirty marking, and the signal policy.
- Declare outcomes explicitly: unchanged, metadata-only, already-presented (a preview already showed the change), one or more bounded rectangles, the whole document, or a link-invalidating change. A no-op records nothing, runs no compositor, bumps no revision, and emits no signal.
- Migrate the ten duplicated local commit helpers and every remaining hand-written `record`/`recomposite`/`clear_link_sets` tail, including multi-rectangle paint commits and the record-only preview commits, onto the transaction.
- Stop engine document operations from compositing as a side effect. Resize, canvas size, crop, delete-cropped-pixels, orient, translation, arbitrary rotation, and perspective crop leave the composite untouched; the app composites once, before capture. `translate_layer_active` remains the CPU/GPU oracle.
- Materialize a merged-only document (no layers, merged composite present) as a Background layer before a geometry operation, preserving its alpha, so the resize/crop/rotate transforms the merged pixels instead of leaving a stale or blank composite.
- Keep live preview edits outside the committed path: previews capture no history and still present through the region or full path.

This is a refactor with one deliberate behavior fix (merged-only geometry edits keep their pixels). Rendered pixels for layered documents, the number of history states per command, and the existing region-vs-full signal behaviour stay as they are.

## Capabilities

### New Capabilities

- `document/edit-transaction`: the app-side contract for committing a document edit — declared outcome, single-owner compositing/history/revision sequencing, explicit no-op, one history state per commit, multi-rectangle refresh, and engine operations that do not composite.

### Modified Capabilities

- `document/document-resize`: the engine no longer recomputes the composite after a resize; the app transaction materializes it, and a merged-only document is materialized as a layer before the resize. The resize oracle materializes the composite before writing.
- `document/document-orientation`: the same change for rotate and flip.
- `ui/document-canvas`: the same change for canvas resize; `translate_layer` no longer recomputes, while `translate_layer_active` remains the CPU/GPU oracle and the app's move commit composites through the active backend.

`document/edit-history` and `compositing/described-damage` already require a capture per successful mutating command and the bounded-region/full paths; this change makes the app honour both through one owner without changing either contract.

## Impact

- `crates/pictura-app/src/cxxqt_object/`: new transaction module; ~40 command files migrate their commit tails; the existing helpers (`mutate_layer`, `mutate_meta`, `batch_changed`, `batch_property`, `edit_active`, `edit_target`, `edit_adjustment`, `commit_style`, `commit_edit`, the fills' `apply`) become thin wrappers or disappear.
- `crates/pictura-render/src/document_ops/`: remove the private `recompute` and its nine call sites; add a merged-composite materialization helper used by the geometry operations; update the ops' doc comments and their composite-equality tests and oracles.
- `crates/pictura-render/tests/document_oracle.rs` and the geometry unit tests materialize `doc.composite` explicitly before writing or comparing.
- History, pyramid, damage, canvas-present, and region-blit behaviour are unchanged for layered documents; no C++/Qt production changes are expected.
- No new dependencies. No changes to `docs/` or the main spec corpus beyond the three capability deltas listed above.
- Related: #738 (layer-stack invariants, already merged) gives structural edits one owner below this one; #740 (document session) depends on this interface; #448 and #404 remain preview-path work.
