# Design: Warp Text

## Model

`pictura_core::warp::TextWarp { style: WarpStyle, bend, h_distort, v_distort,
vertical }` is the `TySh` warp descriptor without a box. `TextWarp::default()`
is `WarpStyle::None` and is the identity. `TextWarp::over(bounds) -> Warp`
builds the existing `Warp` over a box, so type and Free Transform share one set
of preset grids. `TypeSpec` gains `warp: TextWarp`. `TypeTool` is unchanged: it
already keeps the descriptor bytes (`warp_desc`), and the codec decodes them on
demand (`decode_type_warp`). That avoids changing the seventeen `TypeTool`
construction sites.

A custom or unknown `warpStyle` decodes as `None` for rendering. The original
bytes still round-trip untouched until the layer is re-authored.

## Rendering (`pictura-render/src/type_warp.rs`)

Warped type follows `render_mapped`, with the glyph outline as the unit
instead of the glyph bitmap:

1. Lay out the text in its untransformed box (`flat_layout`). The warp box is
   that text box: the paragraph box for paragraph type, the ink-line box for
   point type.
2. For each glyph, scale its outline with swash (`Scaler::scale_outline`),
   embolden it the way Faux Bold does for an opened layer, and apply
   `glyph_matrix` (H/V scale, Faux Italic). Then move it to its pen.
3. Flatten each outline segment into pieces no longer than
   `StyleWarp::max_segment` (and at most a quarter pixel of chord error
   before warping). Map every point through the warp, then through
   `spec.matrix`, into layer-rect pixels.
4. Fill the whole mapped path with zeno (swash's rasterizer, already a
   dependency) using the non-zero rule, then colour it. Decorations are
   rectangles: they are subdivided along their length and filled the same way.

Placement: the warped rect is the integer bounds of the mapped outlines plus
one pixel. Computing it is the same geometry pass, so `type_placement` and
`render_type` share one `warped_paths(spec)` helper. The `TySh` bounds stay
the unwarped box, so the caret and the transform box keep their frame.

Anti-alias None still thresholds the coverage, as unwarped type does.

ponytail: the whole outline set is rebuilt on every render (no cache). The
layer-rect size bounds the work, as for unwarped type.

## Refusal

`pictura_render::type_warp_refusal(spec) -> Option<WarpRefusal>` checks, in
order: type on a path (on-path or area), Faux Bold, and a face with no `glyf`,
`CFF ` or `CFF2` table (`font_has_outlines`). The dialog is not opened when it
returns `Some`. The frame shows a warning naming the reason, in CS6's
"Could not complete the Warp Text command because …" form.

ponytail: CS6's refusal of type on a path is believed but not confirmed from
the CS6 Help. It is kept because the on-path and area renderers do not apply a
warp.

## Bridge

`TypeSetting` gains `warp_style: QString` (the `WarpStyle::id`, empty for
None; C++ value-initializes the struct, so a new setting has no warp),
`warp_bend`, `warp_h`, `warp_v`, and `warp_vertical`. `spec()` sets
`TypeSpec::warp` from them, and `type_layer_setting` fills them from the layer.
The Type tool's edit session unpacks the setting into its `TypeFrame` and packs
it back for every preview and commit. That copy moves into
`TypeFrame::load` / `store`, a pure move that also keeps `tool_type.cpp` under
its cap. The moved copy carries the warp fields, so editing keeps and previews
the warp.

ponytail: while a warped layer is edited, the caret, the selection highlight,
and the transform box stay in the unwarped frame. Only the glyphs are drawn
warped.

A new bridge module, `type_warp.rs`, keeps `type_tools.rs` under its 1200-line
cap. It holds:

- `type_warp_refusal(view, path) -> i32`: 0 none, 1 path, 2 faux bold,
  3 no outlines, −1 not a type layer.
- `type_warp_preview(view, path, style, bend, h, v, vertical) -> bool`:
  re-sets the layer with no history state. The first preview stashes the
  layer as it was (`warp_text_original` on the view state).
- `type_warp_cancel(view) -> bool`: puts the stashed layer back. Cancel then
  restores an opened layer's own pixels and `TySh` exactly, instead of
  re-rendering them.
- `type_warp_commit(…) -> bool`: re-sets the layer, drops the stash, and
  records one "Warp Text" state. History captures states only on record, so
  undo returns to the pre-dialog state even after previews.

## Dialog (`warp_text_dialog.{h,cpp}`)

`QDialog` with a Style combo carrying `WarpStyle::id` data. It uses the same
list and order as Free Transform's warp menu, through a shared
`warp_styles.h` table instead of a copy, minus Custom. Below it come
Horizontal/Vertical radio buttons and three `QSpinBox` + `JumpSlider` rows.
With Style None the orientation and values are disabled, as in CS6.
Choosing a preset while the style was None and the bend is 0 sets Bend to +50.
Every user change emits `changed`.

The frame opens the dialog with `open()` (window-modal, not blocking) and
connects `changed`, `accepted`, and `rejected` to preview, commit, and cancel.
A refusal opens a non-blocking warning `QMessageBox` named `warpTextRefusal`.
The type tools' options bars carry a Warp Text button (the existing
`options.warp` icon) that dispatches the same command. The frame enables it
with the menu row.

## Layers panel

The row snapshot gains `warped`, set for a type layer whose decoded warp is
not the identity. The delegate draws the "T" card and, when the row is warped,
an arc under the crossbar in the same stroke.
