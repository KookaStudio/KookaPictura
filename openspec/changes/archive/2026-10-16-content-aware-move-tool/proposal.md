# Proposal: content-aware-move-tool

## Why

The Content-Aware Move tool (issue #13) was catalogued but disabled. photorust
(perfecto25/photorust) ships it as `move_region` over the healing engine
already ported by `healing-brush-tools` and `patch-tool`. This change ports it
onto Kooka's paint engine, selection model, and tool framework, with CS6's own
options (Mode, Adaptation) rather than photorust's later-CC Structure and
Color sliders.

## What Changes

- `pictura_paint::healing::Adaptation` (Very Strict … Very Loose, default
  Medium): the Content-Aware synthesis takes its patch size and search reach
  from it. Medium keeps the previous fixed values, so Spot Healing and Patch
  are unchanged. The mapping is inferred (Adobe's is undocumented).
- `pictura_paint::healing::move_layer` + `MoveOptions`: Move copies the
  selection's pixels verbatim to the drag target and rebuilds the hole
  content-aware; Extend copies and keeps the original.
- `cxxqt_object/healing.rs`: `content_aware_move`; the selection follows the
  moved pixels inside one `"Content-Aware Move"` history state.
- `tool_region_drag.{h,cpp}`: the outline-then-drag gesture, now shared by
  `tool_patch.cpp` and the new `tool_contentawaremove.cpp`.
- Options-bar row: combine buttons, Mode (Move / Extend), Adaptation, Sample
  All Layers (disabled). Catalog row enabled with an arrow cursor whose tip is
  the hotspot; the J cycle adds Content-Aware Move.
- C++ self-test `content_aware_move` (code 538); guard 98 now probes Red Eye.

## Capabilities

### New Capabilities

- `tools/content-aware-move`: the Content-Aware Move tool and its engine.

## Impact

- `pictura-paint` (`healing/content_move.rs`, `healing/synthesis.rs`),
  `pictura-app` bridge and C++ as above.
- No new dependency.

## Provenance

Ported from photorust's `core/src/healing.rs` (`move_region`,
`place_region`), `core/src/document.rs` (`content_aware_move`), and
`shell/src/canvas/CanvasView.cpp` (<https://github.com/perfecto25/photorust>).
Behavioral parity only; Sample All Layers is a `ponytail:` ceiling.
