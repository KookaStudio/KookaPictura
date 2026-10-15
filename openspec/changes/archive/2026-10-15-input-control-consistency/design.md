# Design

## Inset slider arrow (`numeric_field.cpp`)

The suffix is already a child of the value `QLineEdit`, placed by
`layoutSuffix()`. The arrow joins it: `arrow_ = new QToolButton(edit_)`, not a
layout item, and `layoutSuffix()` becomes `layoutInset()`, which

- reserves `edit_->setTextMargins(0, 0, suffixWidth + arrowWidth, 0)` so the text
  never runs under either inset widget;
- clamps the suffix short of the arrow;
- pins the arrow into `lineEditFrame(edit_)` (the edit's rect inset by its frame
  border), spanning the inner height, and raises it.

`reserveWidth()` grows the fixed width by the arrow's width, and re-runs after
the arrow is built. The arrow is `checkable`; `showPopup()` checks it and the
popup's `QEvent::Hide` filter clears it, so the `:checked` stylesheet state
means "slider open".

## Chevron arrows (`theme.cpp`, `assets/ui`)

`SpinArrowStyle::drawPrimitive` draws `icon("chevron.down"|"chevron.up", palette
foreground)` for spin-box and menu-button arrow primitives (the numeric field's
arrow is a real `QToolButton` icon, so it is tinted the same way).

A stylesheet `image:` cannot tint, which is why the old caret shipped as a
fixed-colour asset and why it could not follow the light family. Rather than
fight `QStyleSheetStyle` (styling `QComboBox::drop-down` without `::down-arrow`
suppresses the arrow; leaving `::drop-down` unstyled makes `QWindowsStyle` draw a
raised bevel), the select keeps a flat `::drop-down` cell plus a
`::down-arrow`/`::menu-indicator` image. The image is a per-family baked chevron
because the engine cannot tint it, but the foreground colour is constant across
the four brightness levels (`theme.cpp` shades only surfaces), so the baked
files are pixel-identical to a tint. `${selectChevron}` picks the file in
`Theme::styleSheet(family, level)`.

## One input family (`theme.cpp`)

One rule lists `QLineEdit, QSpinBox, QDoubleSpinBox, QComboBox, QPlainTextEdit,
QTextEdit` with the same `${inputBg}`/`${inputBorder}`/focus/disabled tokens and
`font-size: ${inputFontSize}px`; `QComboBox QLineEdit` neutralises the editable
combo's inner line edit so it does not double-frame. The options-bar
`#workspaceSwitcher` `QToolButton` takes the same surface and focus treatment,
so it reads as a select. `${inputFontSize}` is substituted at build time (base −
1) exactly as `${tabFontSize}` already is (base − 2).

The per-`#numericField`/`#percentField` line-edit and tool-button rules are
dropped: the generic `QLineEdit` and `QLineEdit QToolButton` rules now cover
them, keeping one definition per primitive.

## Alternatives

- **A `QFrame` separator widget** for the arrow divider: rejected — the button's
  own `border-left` already draws it, one less child to position.
- **Baked light/dark chevron assets** selected by family: rejected — two assets
  per family per direction that still cannot track the four brightness levels;
  the tinted style draw is the single source.
