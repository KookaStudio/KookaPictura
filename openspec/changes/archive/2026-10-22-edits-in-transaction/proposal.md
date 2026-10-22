# Proposal

## Why

The edit transaction (`PictureView::edit`) owns compositing, history capture and
notification, but the mutation it commits usually happens *before* it is called:
106 of the 135 `.edit(...)` closures in `crates/pictura-app/src` assert the
outcome (`|_| Edit::X` or a pre-computed `|_| outcome`) instead of deriving it,
so the transaction cannot tell an edit that changed nothing from one that
changed everything. On the C++ side a committed edit whose outcome notifies
already emits `changed`, which the frame turns into a full refresh, yet many
menu handlers ask for a second one: `frame_menus*.cpp` carries 115 bare
`refresh();` statements. The transaction therefore does not yet decide *whether*
anything changed or *how often* the window is refreshed, and five bridge sites
emit `changed` twice on top of that.

## What Changes

- **The layer-tree family runs its mutation inside the transaction's closure.**
  Arrange, reverse, stamp, lock-group, merge, merge visible, merge clipping mask,
  flatten, the three background conversions, layer-via copy/cut, delete-hidden,
  rasterize, shape rasterize, add solid/gradient fill, smart-object authoring,
  matting and the show/hide-all-effects operation move their mutation into the
  `edit` closure and return the outcome they derived. A mutation the engine
  refuses returns "unchanged", so the transaction commits no history state and
  emits no signal. The align, distribute, clipping, layer-mask, vector-mask,
  hide-layers, lock and paste/clear/scale-style commands already run inside the
  closure through `batch_changed`/`batch_property`; they need no Rust change.
- **A committed edit presents the change once.** A command handler does not call
  the frame's `refresh()` a second time after an edit whose committed outcome
  always notifies. The redundant `refresh();` calls in `frame_menus.cpp`,
  `frame_menus_layer_ops.cpp`, `frame_menus_layer_style.cpp`,
  `frame_menus_align.cpp`, `frame_menus_type.cpp`,
  `frame_menus_smart_object.cpp`, `frame_menus_image.cpp`,
  `frame_menus_select.cpp`, `frame_menus_matting.cpp` and `frame_menus_edit.cpp`
  go, and the command handler no longer triggers a second command-registry
  recomputation for the same edit.
- **The undo label names the edit it will undo.** Edit ▸ Undo reads
  `Undo <label>` for the committed state and Edit ▸ Redo reads `Redo <label>`
  for the next state after an undo, so the notification-driven command-table
  recomputation shows the history state it follows.
- **Five bridge sites stop emitting `changed` twice.** Image ▸ Trim, Image ▸
  Duplicate, Image ▸ Canvas Size, Apply Type Style, the six New/Edit/Delete
  Character/Paragraph Style operations, and Edit Type Layer emit `changed`
  again right after an `edit` that already notified. The five sites cover
  eleven operations. Trim and Canvas Size added a third refresh through their
  handler; both of Image ▸ Duplicate's emissions land before the new view is
  wired to the frame, and the type operations have no handler refresh.
- **Saving a selection notifies.** `save_selection` commits `Edit::Record`,
  which captures history without a notification, so the frame learned about the
  new channel only because the caller refreshed by hand. It becomes a
  notifying metadata commit, as do the Channels panel's New Channel and Delete
  Channel commands, which carry the same defect.
- **Out of scope for this change.** The filters, image adjustments, replace and
  match colour, clipboard paste and clear, the single-layer property and
  opacity/fill commits, and the layer-visibility eye toggle commit outcomes that
  vary at runtime between a bounded region that does not notify and a full
  document, so the shell cannot know from the command alone whether the
  transaction already presented the change. Reporting that across the bridge is
  a separate change. The remaining mutation-outside-the-closure call sites are
  likewise migrated one family per change. The four sites that still refresh
  and record by hand — adding a shape layer, committing a fill, committing a
  transform session and committing a mask-target filter — are left alone: two
  of them need a bounded region that also invalidates the positional links,
  which no transaction outcome can yet express, and the other two belong to the
  fill and filter families. They change with the outcome that expresses it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/edit-transaction`: the mutation runs inside the transaction's
  closure and the transaction derives the outcome from it; a committed edit
  presents the change to the shell exactly once.
- `document/command-registry`: command state is recomputed from the
  transaction's notification, and the handler adds no second recomputation for
  the same notifying edit.

## Impact

- **Rust bridge** (`crates/pictura-app/src/cxxqt_object/`): the layer-tree
  family's mutations move into the `edit` closure — `layer_arrange.rs`
  (arrange, reverse, stamp), `impl_layers/layers_surface.rs` (lock group),
  `impl_layers_select.rs` (delete hidden), `impl_layers_merge.rs` (merge, merge
  visible, merge clipping mask, flatten, background conversions),
  `impl_layers_create.rs` (new layer, new group, group from layers, layer via
  copy/cut), `impl_layers_rasterize.rs` (rasterize, add solid/gradient fill),
  `impl_layers_smart_object.rs` (convert to smart object, place, replace and
  edit contents, rasterize smart object),
  `impl_layers/smart_object_actions.rs` (reset transform),
  `impl_layers/matting.rs` (remove matte, defringe), `layer_style.rs` (show and
  hide all effects) and `shapes.rs` (rasterize shape). The double notifications
  go — `image_adjust/image_ops.rs`, `image_adjust/image_size.rs`,
  `type_tools.rs`. `impl_selection.rs` and `channels.rs` gain the notifying
  channel commits, and `impl_layers.rs` folds the three unreachable
  `Some(None)` arms (two `Edit::Record`, one `Edit::Meta`) into the
  full-recomposite fallback.
- **C++ shell** (`crates/pictura-app/cpp/`): the redundant `refresh();` calls
  are removed from `frame_menus.cpp`, `frame_menus_layer_ops.cpp`,
  `frame_menus_layer_style.cpp`, `frame_menus_align.cpp`,
  `frame_menus_type.cpp`, `frame_menus_smart_object.cpp`,
  `frame_menus_image.cpp`, `frame_menus_select.cpp`, `frame_menus_matting.cpp`
  and `frame_menus_edit.cpp`. The Channels panel keeps its own `refresh()`
  calls: they are the panel's immediate update, and the frame's panel refresh
  is deferred by the 120 ms `panelRefreshTimer_`.
- **Tests**: a new Qt Test suite `tst_edit_refresh.cpp` counts command-registry
  refreshes per committed edit, using the existing
  `CommandRegistry::refreshCount()` probe; Rust unit tests cover the
  transaction semantics in `edit.rs`, `impl_selection.rs` and `layer_arrange.rs`.
- **Related issue**: this lands part of #743 — a committed edit notifies after
  its history state is captured. The transaction already orders them that way;
  what is fixed here are the five double notifications and the non-notifying
  channel commits that #743 names. The four hand-written commit sites stay open
  with it.
- **No dependency, API or file-format change.** The bridge's method signatures
  are unchanged.
- **No roadmap entry.** The change refactors the refresh path without shipping
  or removing a feature, so `ROADMAP.md` needs no entry.
