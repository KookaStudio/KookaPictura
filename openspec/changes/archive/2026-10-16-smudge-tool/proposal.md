# Proposal: smudge-tool

## Why

The Smudge tool (issue #28) was catalogued but disabled. photorust's
`core/src/smudge.rs` drags a carried patch of pixels along the stroke; the
port follows `docs/03-tools/smudge-blur-sharpen.md`.

## What Changes

- `pictura_paint::smudge`: `SmudgeOptions` (Strength, `RetouchMode`, Finger
  Painting) and the per-dab `SmudgeBrush`: each dab lays down, at Strength ×
  tip coverage, the pixel that sat at the same place under the previous dab,
  then picks up the result (a patch a radius past the dab) for the next one.
  The first dab only picks up; Finger Painting starts the finger loaded with
  the foreground. `Stroke::begin_smudge` runs it, optionally picking up from
  the composite (Sample All Layers); Lock Transparency keeps coverage; 16/32-bit
  documents are refused. The per-dab stroke constructors share
  `Stroke::begin_retouch`.
- `cxxqt_object/paint_tools.rs`: `begin_smudge`; one `"Smudge"` state.
- `tool_retouch.cpp`; the bar (`options_bar_retouch.cpp`) has the tip, Mode,
  Strength 50 %, Sample All Layers, and Finger Painting. Catalog row enabled.
- Qt Test `tst_retouch_tools::smudgeTool`.

## Capabilities

### New Capabilities

- `tools/smudge-tool`: the Smudge tool.

## Impact

- `pictura-paint` (`smudge.rs`, `stroke.rs`), `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/smudge.rs`
(<https://github.com/perfecto25/photorust>); the finger carries a dab-sized
patch rather than a copy of the whole layer. Behavioural parity only.
Ceiling (`ponytail:`): with Sample All Layers the composite is a snapshot taken
at the press, so the smear reaches one dab past what it showed then.
