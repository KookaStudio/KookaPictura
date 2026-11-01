# Proposal

## Why

Review of #583 against Photoshop. Stack Images showed hard region edges where
Photoshop gives a soft blend, because masks alone only swap regions. Photoshop
also adds a `(merged)` layer on top, and its dialog shows illustrations of each
method and a Content Aware Fill Transparent Areas option.

## What Changes

- **Merged layer.** Every run adds `<top layer> (merged)` above the processed
  layers, holding a true multiband pixel blend at document depth. Seamless
  corrections are baked into it, replacing the unmerged `Auto-Blend Tones` Curves
  layers. A later run replaces it.
- **Softer stacks.** Focus regions scale with the image and transitions reach
  more bands.
- **Dialog.** Illustrations under each method, drawn in code, and Content Aware
  Fill Transparent Areas, a post-CS6 option marked non-parity. It works at 8-bit
  only, because the synthesis does.

## Capabilities

### Modified Capabilities

- `document/auto-blend-layers`: output layers, Seamless, the dialog.

## Impact

`crates/pictura-render/src/document_ops/layer_ops/auto_blend/`,
`crates/pictura-app/src/cxxqt_object/auto_blend*`,
`crates/pictura-app/cpp/auto_blend_dialog.*`, `frame_menus_align.cpp`.
