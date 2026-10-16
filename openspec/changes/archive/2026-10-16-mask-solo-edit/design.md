# Design: mask-solo-edit

## Context

See `proposal.md` for motivation and `specs/` for the requirements.

The app is a Rust `staticlib` (`crates/pictura-app`) linked into the C++ Qt
shell. Canvas pixels do not live in C++: for a document, `ImageView` crops the
Rust view's `ViewPyramid` through a `LevelProvider` (`frame_canvas.cpp:23`) →
`PictureView::display_image(level, x, y, w, h)` (`impl_core.rs:682`), which
crops `display_crop` (`impl_core.rs:1016`). The pyramid stores levels below 0 as
premultiplied RGBA and takes level 0 by borrowed argument
(`view_pyramid.rs:155`); a document canvas therefore holds no full-resolution
image, and a paint commit presents through region blits
(`refresh_region` / `refresh_regions`, `impl_core.rs:1134`, `:1058`) that bypass
the provider.

The relevant precedents:

- **Layer solo** (`Alt`+eye) is a *document mutation*: `toggleSolo`
  (`layers_panel_actions.cpp:191`) → `impl_layers.rs:675` →
  `pictura_render::apply_visibility` (`document_ops/layer_ops/properties.rs:33`)
  flips `layer.visible` and records history. Not the pattern to copy.
- **Mask edit target** (plain mask-thumbnail click) is *view-only*: the Rust view
  holds `mask_edit_target: Option<String>` (`state.rs:141`), the bridge pair
  records no history, and the panel reads it back for the focus border. This is
  the pattern mask solo mirrors.
- **Channels** transform the presented pixels with a *canvas-local* override
  (`ImageView::setChannelMask`, `image_view_channels.cpp:8`), never touching the
  Rust composite. This is the scoping precedent: the mask view must not leak
  into Channels/Histogram/Navigator (`channels_panel.cpp:306`,
  `histogram_panel.cpp:123`, `navigator_panel.cpp:296`) — all of which call the
  same `display_image`.

`Alt`+`Shift` on the mask thumbnail is the documented rubylith gesture
(`layer-masks.md:67`, `keyboard-shortcuts.md:702`) and `\` also toggles it;
neither is implemented. The current handler treats `Alt`+`Shift` as a plain
`Shift`-click and *disables the mask*, which is wrong. The mask plane's own
coverage rule is `pictura_render::composite::mask_value` (`composite.rs:1177`):
`mask.data` inside `mask.rect`, `default_color` outside it, `255` when disabled
or data-less (it is `pub(crate)` today).

## Goals / Non-Goals

**Goals:**

- A view-only solo display whose state and pixels live on the Rust view and whose
  *source* is canvas-only, presented through the same pyramid crops and region
  blits as the composite.
- Live correctness: painting/filling the mask while soloed updates the view, as
  it does in CS6 (the Alt-click activates the mask for editing,
  `layer-masks.md:61-75`).
- Presentation cost equal to the composite, at any document size.

**Non-Goals:**

- Vector-mask solo, smart-filter-mask solo (same documented gesture at
  `keyboard-shortcuts.md:701`), and the rubylith overlay.
- The Properties/Masks-panel eye entry (`layer-masks.md:65`).
- A second rendering backend: the mask reuses the existing pyramid/present code.

## Decisions

### D1. Solo state lives on `PictureViewRust`, beside `mask_edit_target`

Add `mask_solo: Option<String>`, plus a display cache `mask_level0:
Option<PixelBuffer>` and `mask_pyramid: ViewPyramid`. A bridge pair
`mask_solo_set(path)` / `mask_solo_get()` in `impl_layers/layer_masks.rs` (the
existing mask bridge, already listed in `build.rs` — no `build.rs` change)
mirrors `mask_edit_target_*` (`layer_masks.rs:251`): a path without a raster mask
resolves to nothing, an empty path clears, a stale target self-clears, and no
history is recorded. Unlike `mask_edit_target`, `mask_solo` *is* cleared in
`reset_edit_state` (`impl_core.rs:908`), and `LayersPanel::clearSolo`
(`layers_panel_actions.cpp:226`) pushes `mask_solo_set("")` so the shared reset
(add/duplicate/group/ungroup/delete, document switch) ends it — this covers a
deleted mask or layer without per-site wiring.

*Alternatives.* C++ state on `LayersPanel` — rejected: the canvas needs the
pixels on every present, so the panel cannot own them. A document field —
rejected: it would mutate and serialize what must not be saved.

### D2. The mask is a canvas-only display source, not a `display_crop` mode

`display_crop` / `display_image` stay **composite-only**, so Channels, Histogram,
Navigator, and the filter/adjustment previews cannot see the mask. Instead the
view gains a canvas-scoped accessor:

- `PictureViewRust::canvas_present_image(level, rect) -> QImage` returns the solo
  mask crop when soloing, else the composite crop.
- A bridge `canvas_display_image(level, x, y, w, h)` exposes it, and
  `frame_canvas.cpp`'s provider `crop` lambda calls it instead of `display_image`
  (one-line change). The level-count/level-size lambdas are unchanged: the mask
  pyramid is built with the same halving rule, so both sources expose identical
  levels.
- The commit/region present paths route their image through
  `canvas_present_image` too, so a region blit shows mask pixels while soloed.

The mask plane for `canvas_present_image` comes from a dedicated mask pyramid
(heading D4), never the composite pyramid.

*Alternatives.* Route `display_crop` (the first cut) — rejected: it leaks the
mask into every `display_image` caller. Push a document-sized `QImage` into
`ImageView` — rejected: a second full-resolution buffer and full-resolution
scaling below 200 % zoom, and a C++ render path a future filter/vector-mask solo
would duplicate.

### D3. The panel selects the gesture with a guarded branch order

In the mask-thumbnail block (`layers_panel_events.cpp:172`) the branch becomes,
in order: `Shift && Alt` → consume (reserve rubylith); `Shift` → toggle enabled
(existing); `Alt` → `setCurrentIndex` (activate the layer), `mask_edit_target_set`,
toggle `mask_solo_set`; else → `mask_edit_target_set` (existing plain click).
Selecting the row is required: `active_mask_target` only routes strokes while the
target's layer is the active one (`helpers.rs:248-254`), so an Alt branch that
"selected no row" would break painting the mask it just activated. Every other
thumbnail path and `clearSolo` clear the solo state.

### D4. The mask display source is built once and kept in sync

`mask_level0` is the document-sized mask plane assembled from the soloed
`LayerMask` with `mask_value` semantics (`composite.rs:1177`; the app reads the
public `LayerMask` fields directly, as `mask_thumbnail_image`
(`helpers_composite.rs:542`) already does). It is a document-sized sibling of that
thumbnail builder: value replicated across RGB, alpha 255, with the off-rect /
disabled / data-less fill following `mask_value`. `mask_pyramid` is
`ViewPyramid::rebuild` from it.

Sync points:

- **Solo enter / full rebuild:** `reset_pyramid` (`impl_core.rs:943`) rebuilds the
  mask source when soloed (a no-op otherwise), so a recomposite keeps the mode.
- **Region edits:** `apply_refreshed_region` (`impl_paint.rs:120`) patches the
  mask level-0 region from the mask's new coverage and calls
  `mask_pyramid.update`, mirroring how it patches the composite. This is what
  makes painting-while-soloed correct without a full rebuild.
- **Solo exit:** drop `mask_level0` and `mask_pyramid`.

### D5. A toggle forces one re-present, without a new frame.cpp hook

`mask_solo_set` bumps `canvas_revision` (present-cache key) and `frame_revision`,
so `frame.refresh()` (`frame.cpp:950`) sees a changed `kFrameKey` and calls
`replaceDocument`, which invalidates the present cache and repaints while keeping
zoom/pan (`image_view_document.cpp:48`). `frame_revision`'s doc comment is widened
to "level-0 rebuilt whole **or the display source changed**". This avoids editing
the size-capped `frame.cpp`. Region edits already repaint via `blitRegion`.

*Alternative.* A canvas-keyed edge in `refresh()` — rejected: `frame.cpp` is at
its 1200-line cap, and the `frame_revision` path already exists.

### D6. The mask context menu is a second `showContextMenu` route

`showContextMenu` (`layers_panel_menu.cpp:150`) already special-cases the eye
column (`showEyeMenu`). The mask thumbnail gets the same treatment: when the
click falls inside `delegate_->maskThumbRect`, `showMaskThumbMenu` is shown
instead of `populateRowMenu`, and — as CS6 does — it selects the row and sets the
mask edit target first, so the menu's commands act on the clicked mask. Building
the menu from a small `MaskMenuSpec` table mirrors `kRowSpecs`: label, enabled
predicate, and command id, with unimplemented rows rendered disabled with the
`— not implemented yet` tooltip. Apply is enabled iff the row has a mask and is
not a Smart Object, matching `apply_layer_mask`'s refusal
(`pictura-render/.../layer_masks.rs:125`).

*Alternatives.* Extend `kRowSpecs` with mask rows — rejected: those rows are
per-kind on the *row* menu, not keyed to the clicked thumbnail; a right-click on
the layer thumbnail must still show the row menu.

### D7. Delete removes the mask; Apply bakes it — no confirmation dialog

CS6 prompts "Apply the mask before removing?" on delete, but this project's
Delete is dialog-free everywhere else and the user chose the same here: the mask
menu's Delete and the Delete key both call `layer_mask_delete_path`
unconditionally. Apply is the explicit "bake it" path and uses the existing
`apply_layer_mask`, which already multiplies the layer's existing alpha by the
mask coverage and clears the mask (`layer_masks.rs:146-157`) — so it complements
any pre-existing alpha rather than replacing it, exactly as CS6 does. The bridge
gains a `layer_mask_apply_path` because today only the active-layer
`layer_mask_apply` exists.

### D8. Mask-to-selection reuses the selection combine path

The three combine items need a selection built from a mask. `pictura-select`
gains `selection_from_mask` modelled on `selection_from_layer_alpha`
(`lib.rs:590`), reading the mask's coverage with the same rule as
`composite::mask_value` (`data` in-rect, `default_color` off-rect, `255`
disabled/data-less), so a soloed view and a loaded selection agree at the mask
border. The bridge method lives in `impl_selection.rs` beside
`select_layer_alpha` because it needs the private `apply_selection`
(`impl_selection.rs:435`) and `combine_mode_from`; the mode string
(`add`/`subtract`/`intersect`/`new`) is the same one the selection tools
already pass.

## Risks / Trade-offs

- **Size caps.** `impl_core.rs` is exactly 1200 lines. → The three composite
  display accessors move to a new `impl_core/display.rs` (plain module, declared
  `mod display;`), freeing room for the hooks; the new bridge stays in the
  already-listed `layer_masks.rs`; the new C++ test seam goes in a new
  `layers_panel_test_solo.cpp` rather than the 1393-line
  `layers_panel_test.cpp`. No file crosses its cap.
- **Solo-enter cost.** Building the mask pyramid is O(document) once, and each
  mask dab patches only the changed region. → Acceptable; the same order as one
  recomposite, and no worse than painting the composite.
- **Outside-rect / disabled fill.** The plane must match `mask_value` or the view
  disagrees with the composite at the mask border. → The app assembler replicates
  `mask_value`'s rule and a Rust test pins it for in-rect, off-rect, disabled, and
  data-less cases.
- **`frame_revision` semantics.** Bumping it for a view toggle is broader than its
  "full rebuild" wording. → Its only reader is `refresh()`'s re-present, which is
  exactly the intent; the comment is updated, and `tst_large_document.cpp`'s
  assertions are about region refreshes and are unaffected.
- **Gesture regression.** `Alt`+`Shift` becomes a no-op (today it disables the
  mask). → Intended: it is the documented rubylith gesture, and disabling must
  not be its side effect.

## Migration Plan

None. The feature is additive and view-only; no persisted state, no schema
change, no rollback beyond reverting the commit.

## Open Questions

- The exact `default_color` a newly created mask uses versus CS6's outside-rect
  display is not independently sourced; the implementation follows
  `mask_value` and the Rust test pins the two together. If CS6 differs, only the
  fill constant changes, not the approach.
