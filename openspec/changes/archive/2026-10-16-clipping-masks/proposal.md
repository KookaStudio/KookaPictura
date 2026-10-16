# Proposal: clipping-masks

## Why

Issue #63: import clipping masks from photorust (`core/src/compositor.rs`).
Kooka stored, displayed, and merged `Layer::clipping`, but the compositor
ignored it, and Layer > Create / Release Clipping Mask were inert menu
placeholders.

## What Changes

- `pictura-render` `composite_clipping.rs` (new): every sibling list (document
  root, pass-through and isolated groups) composites a base and the clipped
  layers above it as one isolated group — the base at full opacity, each
  clipped layer confined to the base's coverage (its content alone: Fill 0 and
  effects do not narrow it), the group composited with the base's opacity and
  blend mode. A hidden base hides its clipped layers; a clipped layer with no
  base composites unclipped. The GPU path declines documents with clipped
  layers (the CPU composites them).
- `layer_ops/clipping.rs` (new): `create_clipping_mask` (not the bottom of a
  container, not the Background), `release_clipping_mask` (a clipped layer with
  the clipped layers above it; a base with every layer clipped to it), and
  their `can_*` predicates.
- `cxxqt_object/clipping.rs` (new bridge): `clipping_create`,
  `clipping_release` ("Create Clipping Mask" / "Release Clipping Mask"),
  `clipping_toggle`.
- Layer > Create Clipping Mask (Ctrl+Alt+G) and Release Clipping Mask act on
  the selected layers; Alt-click on the line between two Layers rows clips or
  releases the upper layer.
- Tests: unit tests for the rules and the compositor, a psd-tools composite
  oracle (`clipping_oracle`), and `tst_layers_panel::clippingMasks`.

## Capabilities

### New Capabilities

- `compositing/clipping-masks`: clipping groups and their commands.

## Impact

- `pictura-render`, `pictura-app` (bridge, C++). No new dependency.

## Provenance

Clipping rules from photorust's `core/src/compositor.rs`; group semantics from
CS6 Help ("take on the opacity and mode attributes of the bottommost layer").
Checked against psd-tools' compositor (max difference 2 levels). Ceiling
(`ponytail:`): the isolated group means a clipped layer's blend mode sees only
the base and the clipped layers below it, not the backdrop under the base;
Dissolve inside a clipping group is approximate.
