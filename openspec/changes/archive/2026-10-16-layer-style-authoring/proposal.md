# Proposal: layer-style-authoring

## Why

Kooka already decodes, draws and round-trips all ten `lfx2` layer effects, but
nothing can author one: every Layer > Layer Style entry is a disabled stub
(`command_tree.cpp`). Issue #109 (layer styles, part of the Layers-panel
umbrella #244) asks for the Blending Options dialog, the effect pages, and the
Layer Style menu. This change is the authoring half: engine setters over the
`lfx2` descriptor, the bridge, a Layer Style dialog ported from photorust, and
the Layer Style menu commands. The Layers-panel `fx` badge and expander, the
panel-bottom `fx` button, Global Light and Create Layers are later changes.

## What Changes

- `pictura-render` gains `layer_style`: every dialog control is a
  `"<effect>.<field>"` number read from and written to the layer's `lfx2`
  descriptor (colours packed `0xRRGGBB`, blend modes and choices as indices).
  Writing a field of an absent effect adds it, off, at CS6's defaults;
  `<effect>.on` switches it on or off. `blending.*` keys write the layer's
  mode, opacity, fill, knockout and the two group flags.
- Copy / Paste / Clear Layer Style, Scale Effects, and Hide / Show All Effects
  as document operations. The copied style includes the Blending Options and is
  app-wide.
- The compositor honours `lfx2`'s `masterFXSwitch`: off draws no effects.
- Effect blend modes accept and write Photoshop's string ids for the newer
  modes (`linearBurn`, `blendSubtraction`, …).
- Bevel & Emboss renders every style: Outer Bevel lights outside the content,
  Emboss straddles the edge, Pillow Emboss sinks the outside; the chisel
  techniques render as Smooth. The interior effects stack in CS6's order, so
  an opaque overlay no longer hides Satin, Inner Glow, Inner Shadow, Stroke or
  Bevel.
- Any dialog run by the shared runner passes the canvas zoom keys through.
- The bevel's height is a float distance-field chamfer (no 8-bit banding), and
  the three techniques round it differently.
- Drop / Inner Shadow and Outer / Inner Glow apply their Noise as a fixed grain.
- Pattern Overlay offers the eight built-in patterns, embedding the chosen one
  in the document's `Patt` block (new `pictura-codec` pattern writer).
  `pictura-paint` becomes a normal dependency of `pictura-render` for that
  pattern set (it was already a dev-dependency; no new external crate).
- Layer > Layer Style > Blending Options… and each effect entry open a new
  `LayerStyleDialog` (ported from photorust): the effect list with checkboxes,
  one page per effect, live canvas preview, OK records one state, Cancel drops
  every edit. Shadow Distance uses CS6's exponential 0–30000 px slider, and a
  canvas drag moves the Drop / Inner Shadow.

## Capabilities

### Modified Capabilities

- `compositing/layer-effects`: every bevel style renders; interior effects
  stack in CS6's order.

### New Capabilities

- `compositing/layer-style-authoring`: authoring the `lfx2` effects and the
  Layer Style commands.
- `ui/layer-style-dialog`: the Layer Style dialog and menu.

## Impact

- `pictura-render` `document_ops/layer_ops/layer_style{,_fields,_tests}.rs`,
  `layer_effects/mod.rs`; `pictura-app` `cxxqt_object/layer_style.rs`, new
  `layer_style_dialog.{h,cpp}`, `layer_style_pages.cpp`,
  `frame_menus_layer_style.cpp`, `blend_modes.h` (moved out of the Layers panel),
  and `tst_layer_style`.
- **Output changes:** an `lfx2` with `masterFXSwitch` off no longer draws its
  effects; effects stored with a Photoshop string-id blend mode now use it;
  Outer / Emboss / Pillow and chisel bevels now render; and a layer with
  several interior effects stacks them in CS6's order. No golden baseline
  changes; three bevel tests that asserted the old no-op were rewritten to
  assert the new behaviour.
- **Oracle:** effect rendering is unchanged; authored descriptors are verified
  by decoding them with the shipped decoders and by a PSD save/reopen test. No
  new self-test checks (rule 11).
