# Proposal: selection-marquee-feedback

## Why

Smoke-testing the selection geometry work (#275) exposed five behaviour gaps:
the first Alt during a drag subtracts instead of mirroring when a selection
exists; the Fixed Ratio box grows past the cursor; Fixed Size ignores the
pointer; the Rectangular Marquee's Style leaks into the Elliptical Marquee; and
the Polygonal Lasso's close radius is fixed in document space, so small
selections are hard to close when zoomed in.

## What Changes

- **Alt during a drag mirrors immediately.** Alt held at the press subtracts
  when a selection exists; with no selection it draws from the centre. The first
  Alt press after the drag has begun SHALL pivot-mirror and SHALL NOT switch to
  subtract. Releasing and pressing Alt again after an Alt-at-press subtract also
  mirrors. The captured combine mode and cursor do not change.
- **Fixed Ratio fits the cursor's drag rectangle.** The selection keeps the
  entered ratio and stays inside the non-degenerate box the cursor draws; the
  limiting axis follows the cursor. A one-axis drag still derives the other axis
  and may extend outside the degenerate box. Rounding is half away from zero.
- **Fixed Size follows the cursor.** The entered size is placed with its
  top-left corner at the current pointer and follows it until release, instead
  of being centred on the mousedown. A fixed-size selection partly or wholly
  outside the canvas is clipped by the rasteriser.
- **Marquee options are per tool.** The Elliptical Marquee gains its own whole
  selection options group, so its Style, ratio, size, combine mode, feather,
  tolerance, and magnetic settings are independent of the Rectangular Marquee.
- **Polygonal Lasso close radius scales with zoom.** The Magnetic Lasso already
  uses the screen-fixed radius; only the Polygonal Lasso changes. The close and
  double-click hit radius is fixed on screen at 6 px, so the document-space
  radius is `6 / max(zoom, 1e-6)`; a 4 px document-space selection no longer
  closes instantly when zoomed in.
- No new dependency. The selection tools' behaviour and history states are
  otherwise unchanged, but the options bar changes for the elliptical.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/shape-selection-tools`: Fixed Ratio/Fixed Size cursor behaviour,
  per-tool marquee options, and the zoom-scaled Polygonal close radius.
- `tools/selection-tools`: the Alt-at-press / Alt-during-drag two-stage rule.
- `ui/tool-options-persistence`: the elliptical marquee's independent options.

## Impact

- `crates/pictura-select/src/geometry.rs` — Fixed Ratio and Fixed Size math.
- `crates/pictura-app/cpp/tool_selection.cpp` — Alt edge handling and the
  Polygonal close radius.
- `crates/pictura-app/cpp/tools.cpp` and `tool_options_json.cpp` — the
  Elliptical Marquee's own options and its persistence. `tools_marquee.cpp` is
  reviewed but unchanged: its `marqueeRectForTest` stays Rectangular-only, and
  the new elliptical tests use the canvas and the options group directly.
- `crates/pictura-app/cpp/tests/` and `selftest_tools_selection.cpp` — updated
  self-test expectations and new Qt Test coverage.
- `docs/03-tools/marquee-selection.md`, `docs/03-tools/lasso-selection.md`, and
  `docs/08-selection/selection-tools-overview.md` — the two-stage Alt rewrite,
  the Fixed Ratio/Size wording, and the screen-fixed lasso close radius;
  `docs/dev/STATE.md`.
