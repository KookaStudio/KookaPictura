# Tasks: layer-style-authoring

## 1. Engine

- [x] 1.1 Add `layer_style` (`layer_style_value` / `set_layer_style_value`) over the `lfx2` descriptor with the ten-effect field table and the `blending.*` options.
- [x] 1.2 Add Copy / Paste / Clear Layer Style, Scale Effects, and Show / Hide All Effects.
- [x] 1.3 Honour `masterFXSwitch` in `decode_layer_effects`; accept Photoshop's string-id effect blend modes.

- [x] 1.4 Render Outer Bevel, Emboss and Pillow Emboss; chisel as Smooth; stack the interior effects in CS6's order.

- [x] 1.5 Float distance-field bevel height; distinct technique roundings; soften highlight and shadow apart.
- [x] 1.6 Apply `Nose` to the shadows and glows.
- [x] 1.7 `Patt` pattern writer; Pattern Overlay picks and embeds a built-in pattern.

## 2. App

- [x] 2.1 Add the `layer_style` bridge (value, live set, commit, cancel, copy / paste / clear, scale, show / hide all).
- [x] 2.2 Port photorust's `LayerStyleDialog` as `layer_style_dialog.{h,cpp}` + `layer_style_pages.cpp`; move the blend table to `blend_modes.h`.
- [x] 2.3 Wire Layer > Layer Style in `frame_menus_layer_style.cpp`.
- [x] 2.4 Exponential shadow Distance slider; canvas drag moves the Drop / Inner Shadow.
- [x] 2.6 Pattern Overlay page gets a Pattern menu.
- [x] 2.5 Canvas zoom keys pass through any dialog run by `runDialog`; run the Layer Style dialog under it.

## 3. Verification

- [x] 3.1 Rust tests: defaults, per-effect decoder round-trip, render checks, PSD save/reopen, copy/paste/clear, hide all, scale, blending options, refusals.
- [x] 3.2 Add the `tst_layer_style` Qt Test suite.
- [x] 3.3 Run `bash scripts/verify-fast.sh` and `openspec validate --all --strict`.
