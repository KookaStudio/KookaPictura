# Proposal

## Why

Issue #583, parity gap LAY-5. Edit > Auto-Blend Layers has no command or code.
`docs/05-layers/align-and-distribute.md` specifies it as a mask generator: one
raster layer mask per processed layer, for a panorama or an image stack, with an
optional tonal correction. Adobe's algorithm is unpublished, so this is
behavioural parity only and is validated by image-difference criteria.

Seamless Tones And Colors must not change layer pixels, so the correction is a
clipped adjustment. Clipped adjustments are wrong over a soft base today: at
base alpha 0.5 the output alpha is 0.625 and about a third of the correction
lands, which would fade the correction across every generated mask edge.

## What Changes

- **Command.** Edit > Auto-Blend Layers opens a dialog with the Panorama and
  Stack Images objectives and a Seamless Tones And Colors checkbox. It runs on a
  worker with progress and Cancel, and commits as one history state.
- **Seamless.** One clipped Curves layer named `Auto-Blend Tones` per corrected
  layer. A re-run replaces a layer with exactly that name directly above the base.
- **Clipped adjustments.** A clipped non-fill adjustment over a soft base
  changes colour fully and leaves alpha alone.
- **Mask replacement.** `replace_layer_mask` drops the retained native mask
  plane so a replaced mask gates the composite on 16- and 32-bit layers.

Content Aware Fill Transparent Areas is post-CS6 and out of scope. Video and 3D
layers cannot be refused yet because Kooka has no model for them (#1110).
Auto-Align stays separate (#582): layers blend where they sit.

## Capabilities

### New Capabilities

- `document/auto-blend-layers`: the command, its availability, objectives,
  correction, cancellation and undo.

### Modified Capabilities

- `compositing/clipping-masks`: clipped adjustments over a soft base.

## Impact

`crates/pictura-render/src/document_ops/layer_ops/auto_blend/`,
`crates/pictura-render/src/composite_clipping.rs`,
`crates/pictura-render/src/document_ops/layer_ops/layer_masks.rs`,
`crates/pictura-app/src/cxxqt_object/auto_blend*`,
`crates/pictura-app/cpp/{auto_blend_dialog,busy_job,frame_menus_align}.*`,
`crates/pictura-script/src/command_catalogue/`, and a new `tst_auto_blend` suite.
No new dependencies.
