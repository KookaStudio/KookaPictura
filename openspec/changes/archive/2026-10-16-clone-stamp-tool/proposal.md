# Proposal: clone-stamp-tool

## Why

The Clone Stamp (issue #17) was catalogued but disabled. photorust
(perfecto25/photorust) ships it as `stamp::snapshot` plus a clone stroke that
composites source pixels through the brush coverage. This change ports it onto
Kooka's paint engine and tool framework (`docs/03-tools/clone-stamp-and-pattern-stamp.md`).

## What Changes

- `pictura_paint::stamp`: `StampSource` (a document-space image read at
  `destination + offset`), `CloneSampling` (Current Layer / Current And Below /
  All Layers), `sample_scope` (the layers a clone composites; Ignore Adjustment
  Layers strips adjustments under All Layers), `layer_surface`, and
  `surface_from_composite`.
- `Stroke::begin_source`: a Brush stroke whose colour at each pixel comes from
  a `StampSource`; a pixel whose source is off the image is left alone. The
  Pattern Stamp and History Brush reuse it.
- Fix: the Brush stroke read a layer without an alpha channel (a Background) as
  transparent, so a transparency-locked Background took no paint and soft
  edges on alpha-less layers replaced rather than blended. Such a layer now
  reads as opaque.
- `cxxqt_object/paint_tools.rs`: `begin_clone_stamp`, snapshotting the source when
  the stroke begins; one `"Clone Stamp"` state per stroke through `end_paint`.
- `tool_stamps.cpp`: Alt-click sets the source point; Aligned keeps the offset
  across strokes, unchecked every stroke restarts at the source; a stroke
  without a source, or at a zero offset, is refused.
- Options-bar row (shared with the Pattern Stamp and History Brush): Size,
  Hardness, Mode (Normal / Dissolve / Behind), Opacity, Flow, Aligned, Sample,
  and Ignore Adjustment Layers (shown with All Layers). The stamps and History
  Brush join the brush size ring and `[` / `]`. Catalog row enabled.
- C++ self-test `clone_stamp_tool` (542); `shift_plain` (117) asserts the S
  cycle, `keys_shown` (116) now probes the Y group, and the unimplemented-tool
  guard (98) probes the Art History Brush.

## Capabilities

### New Capabilities

- `tools/clone-stamp-tool`: the Clone Stamp and the source-stroke engine.

## Impact

- `pictura-paint` (`stamp.rs`, `stroke.rs`), `pictura-app` bridge and C++.
- No new dependency.

## Provenance

Ported from photorust's `core/src/stamp.rs`, `core/src/brush.rs`
(`composite_source_onto`), `core/src/document.rs` (`begin_clone_stroke`),
`core/src/bridge.rs` (the aligned offset), and `shell/src/canvas/CanvasView.cpp`
(`clonePress`) (<https://github.com/perfecto25/photorust>). Behavioural parity
only. The Clone Source panel (five sources, scale / rotate / flip / offset,
overlay) and the on-canvas source crosshair are `ponytail:` ceilings.
