# Proposal

## Why

Issue #1103 fixed a proxy-less smart-object base showing its clipped layers over
the base's transparent margin. Moving a layer inside or around that clipping run
then exposed a second defect: the damage bound for a clipped member was the
union of the member and the whole clipped run above it, so a full-canvas clipped
member forced a full-canvas damage and a full recomposite on the first reorder.
Separately, moving an inert or hidden layer still composited the document even
though it cannot change the picture.

Two follow-on gaps surfaced while bounding that damage:

- Releasing a clipping mask on a non-top clipped member under-bounds: the release
  frees the clicked member **and every clipped layer above it**, but the app
  reported damage only for the clicked path. A freed full-canvas layer above can
  render outside the damage.
- A content-neutral move was skipped even in a knockout document. Moving the
  bottom layer (or a group-entry backdrop) reassigns the knockout base even when
  the moved node draws nothing, so the stored composite goes stale.

## What Changes

- Bound a clipped member's influence by its base's reach instead of the run
  union, keeping the conservative union for an unbased run or a Background base.
- Report every layer Release Clipping Mask frees, not only the clicked path.
- Treat a move of an inert or hidden layer as content-neutral only when it is not
  a clipping base on either side and neither document contains a knockout.
- A content-neutral move skips the compositor but still clears the positional
  links, updates the selection, and records one history state. A live gesture or
  an uncommitted preview keeps the full recomposite.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/described-damage`: the influence of a clipped layer is its base's
  reach; a move or clipping-flag change reports both sides; a clipping-mask
  release reports the whole freed run; a move that draws nothing is
  content-neutral and skips the compositor.

## Impact

- `crates/pictura-render/src/damage.rs` (`content_influence`, `base_below`)
- `crates/pictura-render/src/document_ops/layer_ops/clipping.rs`
  (`clipping_release_paths`)
- `crates/pictura-render/src/document_ops/layer_ops/neutral.rs`
  (`move_is_content_neutral`)
- `crates/pictura-app/src/cxxqt_object/impl_layers.rs` (`move_edit`, move paths)
- `crates/pictura-app/src/cxxqt_object/clipping/imp.rs` (`clipping_release`)
- `crates/pictura-app/src/cxxqt_object/edit.rs` (`Edit::Inserted` with a pending
  preview)
- `crates/pictura-render/src/lib.rs` and `document_ops` export
  `move_is_content_neutral` and `clipping_release_paths`.

No dependency or codec-format change. The public API gains
`move_is_content_neutral` and `clipping_release_paths`.
