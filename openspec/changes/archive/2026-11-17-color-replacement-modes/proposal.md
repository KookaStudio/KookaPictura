# Proposal: color-replacement-modes

## Why

The docs→code parity audit (TOOL-23, #547) records two divergences from the
Color Replacement contract (`docs/03-tools/color-replacement.md`):

- `pictura-paint::replace::ReplaceMode` offers only Hue, Saturation, Color, and
  Luminosity. CS6's Mode menu is the full paint-mode list; the current menu
  cannot express the modes the Help describes.
- `begin_color_replacement` has no document-mode gate. CS6 refuses the tool on
  Bitmap, Indexed, and Multichannel documents; today the app offers it there and
  would silently paint the normalized RGB working copy.

## What Changes

- Replace the four-value `ReplaceMode` with the 25-entry CS6 paint-mode list
  (`Normal` through `Luminosity`, including `Behind` and `Clear`) in menu order,
  reusing `pictura_core::blend::blend` for the separable and non-separable
  formulas. `Behind` is a no-op because replacement never touches transparency;
  `Clear` erases matched pixels to transparency.
- Add the mode name and 32-bit availability table to the paint bridge so the
  options bar builds its Mode combo from the engine instead of a second copy.
  A 32-bit document offers only the paint modes the 32-bpc contract keeps.
- Gate Color Replacement on the document's effective colour mode:
  `begin_color_replacement` refuses, the handler reports the restriction, the
  toolbox flyout entry is disabled, and an active Color Replacement tool falls
  back to Move when a document switch or mode conversion makes it unavailable.
- Add Qt Test coverage for the mode menu, the 32-bit filter, and the document
  gate, and engine tests for the new modes.

Not in scope: Opacity/Flow on this tool, the 32-bit blend math beyond the
already-documented mode subset, and a per-document availability framework for
tools other than Color Replacement.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/color-replacement-tool`: the engine applies the full 25-mode CS6 paint
  list, including the `Behind` no-op and `Clear` alpha erasure; the options bar
  offers that list with `Color` default and filters it for 32-bit documents; the
  tool is unavailable on Bitmap, Indexed, and Multichannel documents and refuses
  without converting them.

## Impact

- `crates/pictura-paint/src/replace.rs`: the mode enum, blend dispatch, `Clear`
  alpha, `Behind` no-op, and tests.
- `crates/pictura-app/src/cxxqt_object/paint_tools.rs` and
  `paint_tools/imp.rs`: the document gate, mode names, 32-bit availability, and
  a bridge test.
- `crates/pictura-app/cpp/tool_catalog.cpp`, `tools.cpp`, `toolbox.cpp`,
  `toolbox.h`, `tool_colorreplacement.cpp`: document-aware tool availability and
  the refusal message.
- `crates/pictura-app/cpp/options_bar.h`, `options_bar.cpp`,
  `options_bar_paint.cpp`: the full Mode combo and its 32-bit rebuild.
- `crates/pictura-app/cpp/selftest_paint_tools.cpp`: the existing check's mode
  indices.
- `crates/pictura-app/cpp/tests/tst_color_replacement.cpp` (new) and
  `CMakeLists.txt`.
- `ROADMAP.md`: tick #547.
- No new dependency; no PSD field changes.

## Provenance

The mode list and the Bitmap/Indexed/Multichannel restriction are from the
public Photoshop CS6 Help (`docs/03-tools/color-replacement.md`,
`docs/03-tools/brush-and-pencil.md`). The blend formulas are W3C *Compositing
and Blending Level 1*, already implemented in `pictura-core::blend`. Behavioral
parity only: Adobe's per-pixel replacement math remains closed.
