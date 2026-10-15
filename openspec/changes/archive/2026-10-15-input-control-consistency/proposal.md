# Proposal

## Why

The design system (`ui/design-system`) tokenised the standard controls but left
three inconsistencies in the input family. The numeric field's slider popup is
opened by a `QToolButton` that sits **outside** the value box — a layout sibling
— so it reads as a separate control; its glyph is the native `Qt::DownArrow`
rather than the Lucide chevron used elsewhere. Combo boxes still draw a
fixed-colour triangle asset (`assets/ui/combo-caret*.svg`) that cannot follow the
theme family, so it disappears on the new light ramp. And the inputs, selects,
and the options-bar workspace switcher disagree on surface, border, and text
size. Closing these makes the input family one definition, as the capability
requires.

## What Changes

- **The popup slider arrow becomes part of the input.** In `NumericField`, the
  arrow is reparented into the value `QLineEdit`, flush to its inner right edge,
  separated by a token-coloured hairline, and it darkens (`:checked`) while the
  slider popup is open. It shows the Lucide chevron.
- **One chevron arrow.** The numeric field's arrow and the spin-box arrows are
  the Lucide `chevron.{down,up}`, tinted at render time. The style
  (`SpinArrowStyle`) draws the spin-box and menu-button arrows; the select uses
  the same glyph because the stylesheet engine cannot tint an SVG, so the select
  chevron ships as a per-family asset (`assets/ui/select-chevron{,-light}.svg`)
  — equivalent to a tint, since the foreground colour is constant across the
  brightness levels. The old fixed-colour `assets/ui/combo-caret*.svg` are
  removed. The select's `::drop-down` cell is flat and reserves the value's
  trailing space, so the chevron never overlaps the value (worst before on the
  workspace switcher, which is styled to match).
- **The checkbox tick is restored.** The tokenised `QCheckBox::indicator` rules
  replaced the native indicator without a tick; a white check
  (`assets/ui/checkbox-check.svg`) is set on the checked indicator.
- **One input surface and metric font.** Text inputs, editable inputs, selects
  (combo and font combo), the numeric field, and the workspace switcher share
  one background, border, focus, and disabled treatment, and one text size:
  one pixel under the application default (a new `${inputFontSize}` substitution
  alongside `tabFontSize`).

Not breaking: the token keys and the dark palette values are unchanged; the
slider popup's behaviour (open on the arrow, step keys, commit on hide) is
unchanged.

## Capabilities

### New Capabilities

<!-- None: this change extends the existing capability. -->

### Modified Capabilities

- `ui/design-system`: new requirements cover the inset slider-arrow chevron and
  the shared input-family surface, arrow, and text size.

## Impact

- `crates/pictura-app/cpp/panels/numeric_field.{h,cpp}` — the inset arrow, its
  chevron icon, and the open-state.
- `crates/pictura-app/cpp/theme.cpp` — `SpinArrowStyle` draws the tinted
  chevron; the input-family QSS and the `${inputFontSize}` substitution.
- `assets/icons/chevron.{down,up}.svg` (new) + `lucide-map.json` + `pictura.qrc`;
  `assets/ui/combo-caret{,-disabled}.svg` removed; `assets/PROVENANCE.md`.
- `crates/pictura-app/cpp/tests/tst_design_system.cpp` and
  `tst_layers_panel_fixes.cpp` — arrow-inset check and the combo-chevron shape
  check.
- No new dependency, no engine-crate or cxx-qt bridge change.
