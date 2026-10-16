# Proposal: patch-tool

## Why

The Patch tool (issue #12) was catalogued but disabled. photorust
(perfecto25/photorust) ships it as `Document::patch_selection` over the same
healing engine this repo already ported for the Spot Healing Brush and Healing
Brush (`healing-brush-tools`). This change ports it onto Kooka's paint engine,
selection model, and tool framework.

## What Changes

- `pictura_paint::healing::patch_layer` + `PatchOptions`: heal the active
  pixel layer through a document-sized selection mask — Source (repair the
  selection from the dragged-to area), Destination (apply the selection where
  it is dragged), Transparent (`Transfer::TextureOnly`), and Content-Aware
  (rebuild the selection in place, ignoring the drag).
- `cxxqt_object/healing.rs`: `patch_selection`, one `"Patch Tool"` history
  state per patch.
- `tool_patch.cpp`: CS6's two-step gesture — a drag outside the selection
  traces a freehand outline; a drag from inside previews the outline at the
  offset and patches on release.
- Options-bar row: selection combine buttons, Patch (Normal / Content-Aware),
  Source / Destination, Transparent, Use Pattern (disabled). Catalog row
  enabled; the J-group flyout cycles Spot Healing → Healing → Patch.
- C++ self-test `patch_tool` (code 537); the unimplemented-tool guard (98) now
  probes Content-Aware Move.

## Capabilities

### New Capabilities

- `tools/patch-tool`: the Patch tool and its engine entry point.

## Impact

- `pictura-paint` (`healing/patch.rs`), `pictura-app` bridge and C++ as above.
- No new dependency.

## Provenance

Ported from photorust's `core/src/document.rs` (`patch_selection`) and
`shell/src/canvas/CanvasView.cpp` / `shell/src/MainWindow.cpp` (the gesture and
bar) (<https://github.com/perfecto25/photorust>). Behavioral parity only: the
Poisson / patch-synthesis approximations are those of `healing-brush-tools`;
Adaptation, Sample All Layers, and Use Pattern are `ponytail:` ceilings.
