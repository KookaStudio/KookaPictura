# Proposal: background-eraser-tool

## Why

The Background Eraser (issue #22) was catalogued but disabled. photorust's
`core/src/erase.rs` has it; the port follows `docs/03-tools/eraser-tools.md`
(TOOL-023).

## What Changes

- `pictura_paint::replace`: the match (`match_strength`) and the per-dab flood
  (`reachable`) become shared free functions; Color Replacement is unchanged.
- `pictura_paint::eraser`: `BackgroundEraseOptions` (Sampling, Limits,
  Tolerance, Protect Foreground Color) and a per-dab `BackgroundEraser`, run as
  `StrokeKind::BackgroundErase`. Each dab samples the colour under the
  crosshair (continuously, once, or the background swatch; a cleared pixel is
  never sampled), finds matching pixels within the dab (connected, or not, or
  stopping at edges), and multiplies their alpha down; a protected foreground
  colour is kept. It overrides Lock Transparency. `ensure_alpha` gives a
  Background turned layer an opaque alpha channel.
- `cxxqt_object/paint_tools.rs`: `begin_background_eraser`, one `"Background
  Eraser"` state; on the Background the stroke first makes it a layer.
- `tool_eraser.cpp` handler (the Eraser and Background Eraser share the drag);
  the bar (`options_bar_erase.cpp`) has the tip, Sampling, Limits, Tolerance
  50 %, and Protect Foreground Color. Catalog row enabled; it joins the size ring.
- C++ self-test `background_eraser_tool` (550). The unimplemented-tool guard
  (98) and `keys_shown` (116) now probe the G group.

## Capabilities

### New Capabilities

- `tools/background-eraser-tool`: the Background Eraser.

## Impact

- `pictura-paint` (`eraser.rs`, `replace.rs`, `Stroke`), `pictura-app`.
- No new dependency.

## Provenance

Ported from photorust's `core/src/erase.rs` and `core/src/sample.rs`
(<https://github.com/perfecto25/photorust>). Behavioural parity only. The
matched edge is always softened (CS6 has no Anti-alias control here). No edge
colour extraction and no pen-pressure Size / Tolerance controls — `ponytail:`
ceilings.
