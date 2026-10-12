# Design

## Context

`Theme` (`crates/pictura-app/cpp/theme.{h,cpp}`) already owns Fusion, the
4-level dark ramp, and the global stylesheet. Its token table is private to
`styleSheetFor`; the ramp struct is not addressable from outside. `main.cpp`
has a hand-rolled argument loop with `--self-test`, `--headless`,
`--interop-probe`, and `--control`. The app has one shared-widget folder
(`cpp/panels/`) holding `AngleDial`, `RampSlider`, `SpectrumBar`,
`CurveWidget`, and `NumericField`/`PercentField`, but no shared colour-swatch
widget: ten call sites each paint a swatch with their own `setStyleSheet`.
Static surface colours also leak into per-widget stylesheets (drop
indicators, preview panes, bold labels). `ui/workspace-shell-polish` pins the
default-level palette and the tool-slot metrics, which this design reuses
unchanged.

## Goals / Non-Goals

**Goals:**

- One token list that feeds the stylesheet, the JSON dump, and the generated
  guide.
- One global definition per standard primitive, with tokenised states.
- A shared swatch widget for data-driven colour fills, so dynamic colour never
  needs a per-widget stylesheet.
- A generated, idempotent HTML reference and a Qt Test that checks the
  contract.

**Non-Goals:**

- Changing the palette values, the four levels, or the slot metrics
  (`ui/workspace-shell-polish` owns them).
- Restyling custom-painted widgets that already own their colours
  (`AngleDial`, `CurveWidget`, the toolbox swatches) or the icon engine.
- A Storybook-style runtime, a web dependency, or engine/bridge changes.

## Decisions

**D1 — Token accessor shape.** `Theme::tokens(int level)` returns
`QVector<ThemeToken>` with `{key, value, color}`; `key` is the stylesheet
substitution key (`window`, `panelBorderWidth`, …), `value` is `#rrggbb` for
colours and a decimal string for metrics. Derived shades (menu disabled text,
separator light, alternate table background, panel-header border, scrollbar
thumb hover, hover/icon-hover) are computed inside the accessor, so the
stylesheet template keeps using `${key}` and the dump cannot drift. Alternative
considered: a struct with named fields — rejected because every new token then
needs three edits (struct, QSS substitution, dump) instead of one.

**D2 — Dump format.** `--dump-theme` prints
`{"defaultLevel": n, "levelCount": n, "levels": [{"level": i,
"colors": {...}, "metrics": {...}}, …]}` as indented JSON on stdout and exits
0. `QJsonObject` sorts keys, so the output is deterministic without ordering
code. The mode is handled in `main.cpp` after `QApplication` exists (offscreen
is already selected by `--headless`) and before the frame is built, so it never
opens a window or reads the session store.

**D3 — Primitive set and scoping.** The primitives are the controls the issue
lists; each is a global rule keyed by type (`QPushButton`, `QLineEdit`,
`QComboBox`, `QCheckBox`, `QSlider`, `QMenu`, `QTabBar`) or object name where
a surface must be distinguished (`#panelTabBar` vs `#documentTabBar`,
`#colorSwatch`, `#numericField`/`#percentField`, `#panelHeaderBand`).
`ColorSwatchWidget` is the one new widget: a `QPushButton` subclass (keeps
`findChild<QPushButton*>` and `clicked` working at the `filterColorSwatch`
site) whose `paintEvent` fills the data colour and draws the canonical frame
from the palette; `setActive` gives the Color panel's thicker active frame.
It also mirrors `setColor` into the `color` dynamic property, which existing
tests read. Alternative considered: a stylesheet helper function — rejected
because the spec requires the shared widget and a helper still leaves a
per-widget stylesheet.

**D4 — Static literals move to QSS.** Drop indicators (`#2a7fff`) get a new
constant `accent` colour token; preview panes use `inputBg`/`inputBorder`;
bold titles/tabs use `#dialogTitle` / `#sectionTitle` / `#sectionTabLabel`;
menu padding and the filter-gallery zero-width indicator join the global
stylesheet under their existing object names. `statusOptionsButton`'s
per-widget stylesheet is deleted because the global rule already exists.

**D5 — Style guide generation.** `scripts/gen-styleguide.py` runs
`./build/pictura --headless --dump-theme`, parses the JSON, emits one CSS block
per level (`:root[data-level="i"] { --key: value; }`) plus a token table and a
primitive/state gallery that references `var(--key)`. Output is written
atomically and compared with the previous file, so a re-run with no theme
change is byte-identical. The page uses `data-level` switching (radio buttons +
CSS) to show all four levels without JavaScript frameworks.

**D6 — Test contract.** `tst_design_system` runs offscreen like the other
suites. It (a) compares `Theme::tokens(level)` colours with the palette roles
`Theme::apply` sets, (b) walks the stylesheet for every primitive's selectors,
and (c) instantiates each primitive and checks object name / state round-trips
(checked, disabled, focus, swatch colour, numeric field value). It reads the
token list through the same public accessor the dump uses, so the test is the
drift guard at build time and the HTML script is the drift guard at doc time.

## Risks / Trade-offs

- [Changing the swatch sites changes widget types] → `ColorSwatchWidget`
  derives `QPushButton` and keeps the `color` property and `clicked` signal;
  only `QToolButton`-declared members change type, and no test reads those as
  `QToolButton`. `tst_filter_menu`'s stylesheet assertion moves to
  `swatch->color()`.
- [QSS for `QCheckBox::indicator` replaces the Fusion tick with a filled
  square] → accepted: it is the tokenised primitive; the checked fill uses the
  highlight token and stays legible on the dark ramp.
- [The generated HTML can go stale] → the script regenerates from the binary
  and the Qt Test asserts the same tokens, so a theme edit without regeneration
  is caught in review by the diff (the script is the drift guard named in the
  issue).
- [The dump adds a CLI mode] → the argument is opt-in and handled before any
  window or session code, so interactive launches are unchanged.

## Migration Plan

Land as one change: token accessor first (stylesheet behaviour unchanged), then
the primitive QSS and swatch widget, then the literal fold, then the dump +
script + test. No persisted state or data format changes; rollback is a revert.

## Open Questions

None.
