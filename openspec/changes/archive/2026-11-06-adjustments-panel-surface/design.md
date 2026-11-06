# Design

## Context

See `proposal.md - Why`. `AdjustmentsPanel` (`crates/pictura-app/cpp/panels/adjustments_panel.cpp`)
builds a `QScrollArea` with `widgetResizable(true)` and `NoFrame`, then puts an
unnamed `QWidget` (`gridHost`) holding the nineteen icon buttons inside it. The
panel itself is a page of a `PanelGroup` `QTabWidget`; the theme already paints
`QTabWidget#panelGroupTabs QStackedWidget` and `QWidget#panelColumnContainer`
with `${surface}`.

A `QAbstractScrollArea` viewport has `autoFillBackground(true)` and the
`QPalette::Base` background role. The theme maps `QPalette::Base` to
`${surface-field}` (`#404040` at the default level), which is darker than
`${surface}` (`#4d4d4d`). The grid host is transparent, so the viewport's fill is
what the user sees around the buttons. The Styles panel is a `PlaceholderPanel`,
a plain transparent widget, so it shows the stack's `${surface}`; that is the
reference shade.

## Goals / Non-Goals

**Goals:**

- The Adjustments panel, its scroll viewport, and its grid host render the same
  `${surface}` as the Styles panel.
- The colour comes from the theme token, so it tracks the theme family and
  brightness level instead of being hard-coded in C++.

**Non-Goals:**

- Restyling every panel that uses a `QScrollArea` (Brush, Properties, Swatches).
  The issue names the Adjustments panel; a global scroll-viewport rule would
  change panels whose content is intentionally recessed (for example the
  Swatches grid).
- Changing the icon grid's layout, spacing, or button styling.

## Decisions

### Name the three widgets and style them from the theme

Chosen: give the scroll area, its viewport, and the grid host the object names
`adjustmentsScroll`, `adjustmentsViewport`, and `adjustmentsGridHost`, then add
three theme rules that paint each with `${surface}`. Naming the viewport avoids
relying on Qt's internal `qt_scrollarea_viewport` object name, and it keeps the
rule scoped to this panel.

Alternative considered: `scroll->viewport()->setAutoFillBackground(false)` in
C++. Rejected: the viewport would then depend on the parent stack being painted
`${surface}`; it works in the docked column but not in a floating overlay or a
future host that does not paint the surface, and it does not make the grid host
explicit.

Alternative considered: hard-code `QColor(0x4d, 0x4d, 0x4d)` in the panel.
Rejected: it would not follow the light theme or a brightness change.

### Test against a real window and a sibling panel

Chosen: `tst_adjustments_panel::panelSurfaceMatchesStylesPanel` shows a real
`PicturaMainWindow`, activates the Adjustments tab, grabs the window and samples
a background pixel inside the panel, then activates the Styles tab in the same
group, grabs and samples again, and compares both to the theme's `surface`
token. Sampling the top-left of each panel lands in the grid margin for
Adjustments (the layout's 6 px margin) and in the empty area above the centered
placeholder label for Styles, so no button or text contaminates the sample.

`tst_adjustments_panel::surfaceFollowsTheme` re-applies the theme at another
brightness level, processes events so the style sheet repolishes the widgets,
and asserts the sampled Adjustments background equals the new `surface` token;
it restores the default theme at the end.

## Risks / Trade-offs

- [The Styles tab must exist in the same group as Adjustments] -> The fresh
  session default (Essentials) puts them in one group; the test asserts
  `containsPanel("stylesPanel")` before switching, so a future layout change
  fails loudly instead of sampling the wrong widget.
- [Window grabbing can differ with the device pixel ratio] -> The helper scales
  the logical sample point by the grabbed image's `devicePixelRatio()`.
- [A theme change in the test leaks into later functions] -> The test restores
  `Theme::kDefaultLevel` / `kDefaultFamily` before returning.
