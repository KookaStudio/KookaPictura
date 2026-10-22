# Proposal: Warp Text

## Why

`Type > Warp Text…` is an unwired leaf (`command_tree.cpp`), so type layers
cannot be warped, and a warped type layer opened from a PSD loses its warp the
first time it is edited (`replace_type_layer`'s `ponytail:`). Issue #684 (a
sub-issue of #541) asks for the CS6 dialog: Style, Horizontal/Vertical, Bend and
Horizontal/Vertical Distortion, with the warp staying live on the editable type
layer, and Faux Bold and outline-less fonts refused
(`docs/03-tools/type-tools.md`).

## What changes

- `Type > Warp Text…` and a Warp Text button on the four type tools' options
  bar open a modal `WarpTextDialog` for the current type layer. The dialog
  offers Style (None plus the fifteen CS6 presets), Horizontal/Vertical, Bend,
  Horizontal Distortion, and Vertical Distortion (each −100…+100 %), opens with
  the layer's current warp, and previews live on the canvas. OK records one
  "Warp Text" history state. Cancel restores the layer. Style None removes the
  warp.
- A warped type layer renders by mapping every glyph outline through the warp
  and then rasterizing it, so warped edges stay as crisp as unwarped type. The
  warp is fitted to the text's untransformed box and applied before the layer's
  transform. Underline and strikethrough bend with the text.
- The warp stays live: retyping the text, changing character or paragraph
  attributes, applying a style, converting point/paragraph, and transforming
  the layer all keep it. The Type tool previews a warped layer warped while
  its text is edited.
- PSD: the `TySh` warp descriptor (`warpStyle`, `warpValue`, `warpPerspective`,
  `warpPerspectiveOther`, `warpRotate`) is read into a live warp on open and
  written from the layer's warp on save.
- Refusals: a layer using Faux Bold, a font with no outline data, or type on a
  path (on-path or area type) is not warped; an alert names the reason and the
  layer is unchanged.
- The Layers panel draws a warped type layer's thumbnail as the type "T" card
  with a bent baseline, as CS6 marks a warped type layer.

## Capabilities

- Added: `tools/warp-text`.

## Impact

- `pictura-core`: `TextWarp` (style, bend, distortions, orientation) in
  `warp.rs`; `TypeSpec::warp`.
- `pictura-codec`: decode the `TySh` warp descriptor to a `TextWarp`; author it
  from `TypeSpec::warp`.
- `pictura-render`: outline-warped glyph rendering (`type_warp.rs`), warped
  placement bounds, `type_layer_spec` carrying the decoded warp, a warp
  refusal check, and `font_has_outlines`.
- `pictura-app` bridge: `TypeSetting` carries the warp. New `type_warp_*`
  functions (read, refusal, preview, commit) live in a new bridge module so
  `type_tools.rs` stays under its size cap. The layer-row snapshot gains
  `warped`.
- `pictura-app` C++: `warp_text_dialog.{h,cpp}`, Type-menu and options-bar
  wiring, the Layers-panel thumbnail, and CMake. Tests: a new `tst_warp_text`
  Qt suite and Rust unit tests in core, codec, and render.

## Out of scope

- Custom (mesh) warps on type: CS6 offers only the presets for type.
- Warp for the Type Mask tools: they make selections, not layers.
- Exact parity with CS6's warped outlines: the preset grids are the existing
  measured approximation (`pictura_core::warp`), not Adobe's algorithm.
- The remaining Type menu stubs (Create Work Path, Convert to Shape, and the
  others, #502), and gamma blend and missing-font handling (#685).
