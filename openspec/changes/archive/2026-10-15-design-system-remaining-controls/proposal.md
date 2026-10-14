# Proposal

## Why

The design system (#249, capability `ui/design-system`) tokenised the core
primitives but left five standard controls on raw Fusion and four custom-painted
widgets on hard-coded greys. That drift makes the token contract incomplete and
will surface the moment a light theme is applied, so close it before the theme
and density work begins.

## What Changes

- **Remaining controls become primitives.** Add one tokenised global QSS rule
  per control still drawn raw, for the controls actually in use: group box,
  radio button, progress bar, header view, and a general splitter rule. States
  per the #249 contract; object-named only where a surface differs. Controls
  already tokenised (scrollbars, spin/double-spin boxes, tooltips,
  tree/table/list views, `QPlainTextEdit`) are not touched, and `QTextEdit` is
  omitted because the app never instantiates it.
- **Custom-painted widgets read tokens.** Route the custom-painted widgets that
  still hard-code theme greys — `CurveWidget`, `SpectrumBar`, the histogram
  view, and the navigator thumbnail — through `Theme::tokens`; add tokens where
  a colour has no equivalent and delete the literals. `AngleDial`, `RampSlider`,
  and `ColorSwatchWidget` already read the palette and are unchanged.
- **Literal guard.** Add a guard that rejects theme-like grey colour literals in
  widget paint code, complementing the existing non-goal guards, with a baseline
  allow-list for documented data-colour sites.

Not breaking: the token keys, the dark palette values, and interactive launches
are unchanged.

## Capabilities

### New Capabilities

<!-- None: this change extends the existing capability. -->

### Modified Capabilities

- `ui/design-system`: a new requirement adds the remaining standard controls; a
  new requirement routes custom-painted widgets through tokens; a new
  requirement forbids theme-like greys.

## Impact

- `crates/pictura-app/cpp/theme.{h,cpp}` — five new primitive rules; tokens for
  the custom-painted widgets.
- `crates/pictura-app/cpp/panels/curve_widget.{h,cpp}`,
  `crates/pictura-app/cpp/panels/spectrum_bar.{h,cpp}`,
  `crates/pictura-app/cpp/panels/histogram_panel.cpp`,
  `crates/pictura-app/cpp/panels/navigator_panel.cpp`.
- `crates/pictura-app/cpp/tests/tst_design_system.cpp` — new selectors and a
  paint assertion.
- A guard script under `scripts/` (wired into `scripts/guard.sh`). No new
  dependency, no engine-crate or cxx-qt bridge change.
