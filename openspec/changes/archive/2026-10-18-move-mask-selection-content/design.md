# Design

## Context

See `proposal.md` - Why. The relevant current state:

- `PictureView::move_selection_content` always calls the pixel engine, which
  cuts the active layer's pixels and merges the floating copy back through
  `bake_inputs`; the merge returns a layer with `mask: None`, so the mask is
  applied and lost.
- Brush, eraser, fill, and filters already route through
  `active_mask_target(rust)`, which is `Some(layer path)` only while the active
  layer's raster mask is the edit target.
- The Move tool's selection drag and the selection tools' Ctrl content move
  share `ToolController::beginContentMove` / `releaseSelectionMove`, so there is
  one bridge call site.
- A mask is stored as `LayerMask { rect, data: Option<Plane<u8>>, default_color }`
  in document space. Mask-local edits use `mask_document_coverage`,
  `crop_selection`, and `write_mask_back`.
- The C++ colour state keeps one foreground/background pair per edit target and
  pushes it into `ToolController`; paint APIs already pass the background ARGB
  into Rust. History captures the whole document, masks included.
- `crates/pictura-app/src/cxxqt_object.rs` is exactly at the 1200-line hard cap
  and is not in `scripts/file-size-allowlist.txt`.

## Goals / Non-Goals

**Goals:**

- Move selected mask coverage, blend the vacated area toward the mask background
  luma, support Alt copy, and record one `Move Selection` state.
- Keep the layer's pixels, the mask attachment, and the mask rectangle
  unchanged.
- Make the default mask pair match CS6 (white foreground, black background) so
  the default vacated coverage is black, including after `D`/reset.

**Non-Goals:**

- Whole-layer Move with a mask target and no selection, or moving an unlinked
  mask independently of its layer (the link flag is separate behavior).
- A live mask-content preview during the drag. Unlike the pixel Alt preview,
  a mask Alt move shows only the moving selection outline.
- Growing or re-anchoring the mask rectangle during a move.
- Moving mask content from outside the stored mask rectangle.
- Mask creation on group/adjustment layers: those layers currently have a 0x0
  rect, so a UI-created mask is zero-sized. The routing still runs before the
  pixel-layer guard, so a hand-built non-empty mask works.

## Decisions

### Move mask coverage in a dedicated engine operation

Add `move_mask_selection_content` to the mask-editing engine, taking the
mask-local selection coverage, the offset, the duplicate flag, and the
background luma. It snapshots the stored coverage, builds the base plane, stamps
the moved coverage from the snapshot, and writes the plane back.

*Alternative:* reuse the pixel content-move engine on a temporary grayscale
document. Rejected: the engine clears to transparency and merges through
`bake_inputs`, which has no meaningful transparent state for a mask and drops the
real mask.

### Composite the floating coverage over the cleared base

For a non-duplicate move, every selected source pixel first becomes
`blend(background, selection_coverage, old)`. Then every destination pixel takes
`blend(snapshot[src], selection_coverage[src], base[dest])`. Using the snapshot
for both source and destination makes overlapping source/destination regions
order-independent and matches a floating selection composited over the cleared
mask. Alt (duplicate) skips the clear, so the base is the snapshot.

*Alternative:* binary cut/fill at 255 coverage. Rejected: feathered selections
would harden instead of blending toward the background.

### Keep mask-local coordinates and clip to the mask rectangle

The bridge crops the selection origin to the mask rectangle with the existing
`crop_selection`, then passes a mask-sized coverage plane. Source coordinates
outside that plane are unselected. The mask's `rect` and `default_color` are not
changed.

*Alternative:* treat the region outside the mask rectangle as movable
`default_color` coverage and grow the mask. Rejected: brush, fill, and filters
already treat the mask rectangle as the editable mask document; a move should not
implement a different extent model.

### Route in Rust at the content-move bridge

Check `active_mask_target` at the top of `PictureView::move_selection_content`;
the mask branch runs before the pixel-layer validation and returns before the
pixel post-processing. In particular it does not run the `layer_above` repoint
that follows a pixel Alt duplicate, so the active layer never changes on a mask
move. Keep the pixel path as the fallback.

*Alternative:* branch in C++ on `mask_edit_target_get`. Rejected: it duplicates
the routing rule at the call site and would need repeating for future content
move entry points.

### Use the current mask background swatch

Pass the C++ background ARGB through the bridge and convert it with the existing
`luma_u8`. This keeps the fill swatch-driven like mask fill and mask erase.

*Alternative:* fixed black. Rejected: Photoshop fills with the background swatch;
only the CS6 default happens to be black.

### Keep Alt as a copy inside the mask

Duplicate uses the same destination compositing but keeps the source plane
intact. No layer is inserted, the active layer does not change, and the pixel
clone preview is suppressed by making `begin_selection_duplicate_preview`
return false on a mask target.

*Alternative:* create a new layer for the copy, as the pixel path does. Rejected:
a layer mask has no separate layer to receive a duplicate; Photoshop copies the
coverage within the same mask.

### Translate the selection before either content move commits

`releaseSelectionMove` currently commits a content move from the selection origin
without first applying the release offset. Add `preview_selection_move(dx, dy)`
before the engine call, for both pixel and mask content moves. A release that
arrives without a final pointer-move event then leaves the selection on the
moved coverage, as the history scenario requires.

### Initialize and reset the inactive colour pair as white/black

At startup the image pair is active (black foreground, white background) and the
inactive pair represents the mask. Initialize that pair to white foreground /
black background in the frame. Add a mask-active flag to `ColorState`, set from
the existing `editTargetChanged` handler, and make the toolbox `D`/reset use
white/black while a mask is the target and black/white otherwise. The pair swap
on target change then gives CS6 defaults for masks and restores image colours on
the way back.

*Alternative:* set only the mask background to black while leaving the
foreground black. Rejected: CS6 uses white foreground so a mask brush reveals by
default, and the pair must stay a pair.

### Split the selection-move bridge out of `cxxqt_object.rs`

`cxxqt_object.rs` is exactly 1200 lines. Move the selection-move bridge
declarations (`begin_selection_move`, `preview_selection_move`,
`commit_selection_move`, `cancel_selection_move`, `move_selection_content`) into
a small module under `impl_selection/`, following the free-function bridge
pattern of `impl_layers/layer_masks.rs`. Update the C++ call site to the free
function form. This keeps the new `background` argument from pushing the main
bridge file over the hard cap.

*Alternative:* add `cxxqt_object.rs` to the file-size allowlist. Rejected: the
allowlist only shrinks, and the module boundary already exists for this kind of
bridge growth.

## Risks / Trade-offs

- **Default mask colours change** -> `tst_mask_edit::colorsFollowTheEditTarget`
  and the Delete-fill case in `tst_mask_edit::maskContextMenu` encode the old
  black/white defaults; update both, setting an explicit white background where
  the Delete test needs a revealing fill. Add a reset-to-defaults test.
- **One history snapshot per move** -> masks are single-channel and moves commit
  on release, so the copy cost matches other mask edits.
- **A selection entirely outside the mask records a state** -> the engine still
  succeeds with a no-change plane. This mirrors the pixel content move, which
  records on any non-zero offset; tighten only if it becomes noticeable.
- **Stored mask data may be absent or malformed** -> materialize from
  `default_color` before moving, as the other mask edit paths do. A **passed
  selection plane** of the wrong length is a caller error and refuses.
- **Layer locks** -> mask paint and fill ignore layer pixel/position locks, so a
  mask move ignores them too. Add an engine test pinning that a locked layer's
  mask still moves.
- **Group/adjustment masks** -> the engine does not require a pixel layer, but
  UI-created masks on those layers are currently zero-sized. A hand-built
  non-empty group mask is routed; the zero-rect creation limitation stays out of
  scope.
- **Mask solo view** -> `recomposite` rebuilds the solo plane, so an active solo
  view stays current.
- **Bridge extraction** -> mechanical move; verify the generated header include
  and the C++ call site, and run `check-file-size.sh`.
