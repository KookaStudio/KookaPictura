# Proposal

## Why

Standard controls are constructed raw and styled per call site: dozens of
`setStyleSheet()` calls across the app hard-code colours (`#2b2b2b`, `#555`,
`#000`, `#2a7fff`, …) that bypass `Theme`, so button/field/row states drift and
every new dialog reinvents the same widget. `Theme` already is the token layer
(Fusion + a 4-level dark ramp + tokenised QSS); the design system makes that
contract explicit, single-sourced, documented, and tested.

## What Changes

- **Token contract.** The private token table inside `Theme`'s QSS builder
  becomes a public accessor `Theme::tokens(int level)` covering colours and
  metrics (panel border, chrome width, slot footprint/icon maxima, shade step,
  group divider). The QSS builder and a new headless `--dump-theme` JSON mode
  both consume it, so the stylesheet and the machine-readable ramp cannot
  diverge.
- **Canonical primitives.** One global QSS definition per primitive — text
  button, icon/toggle button, text input, numeric field, combo box, check box,
  slider, panel/document tab, menu, colour swatch, panel header — covering
  rest / hover / pressed / checked / disabled / focus. Small construction +
  signal-wiring helpers are added only where the same code actually repeats.
- **Consistency pass.** Ad-hoc `setStyleSheet()` calls that hard-code static
  colours or font weights are folded into object-name-scoped global QSS and the
  literals deleted. Data-driven colours (chosen-colour swatches, previews)
  are routed through the shared swatch widget and keep dynamic fills.
- **Generated HTML style guide.** `scripts/gen-styleguide.py` runs
  `./build/pictura --headless --dump-theme`, emits CSS custom properties from
  the token JSON, and writes `design-system.html` at the repo root rendering
  every token, primitive, and state across all four levels. Re-running produces
  no diff. The HTML is a demo/reference, not the reuse mechanism.
- **Runnable check.** `cpp/tests/tst_design_system.cpp` (Qt Test) instantiates
  each primitive, asserts its object-name / state contract, and asserts the
  applied palette colours equal `Theme::tokens()`.

## Capabilities

### New Capabilities

- `ui/design-system`: the theme token contract (`Theme::tokens`, `--dump-theme`
  JSON), the canonical primitive stylesheet with its state contract, the
  no-ad-hoc-colour rule, and the generated style guide.

### Modified Capabilities

<!-- None: workspace-shell-polish keeps its palette and metrics requirements;
     this change exposes them through the token contract without changing them. -->

## Impact

- `crates/pictura-app/cpp/theme.{h,cpp}` — token accessor; primitives in the
  global QSS; new object names.
- `crates/pictura-app/cpp/main.cpp` — `--dump-theme` headless mode.
- `crates/pictura-app/cpp/` dialogs, panels, and option bars — remove static
  `setStyleSheet` colours; keep data-driven fills.
- `scripts/gen-styleguide.py`, `design-system.html` (repo root, generated).
- `crates/pictura-app/cpp/tests/tst_design_system.cpp` + tests CMake list.
- No new dependency, no engine crate, no cxx-qt bridge change.
