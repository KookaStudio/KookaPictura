# Spec Delta

## Purpose

One token contract and one canonical definition per standard control, so every
dialog, panel, and option bar draws its buttons, fields, tabs, and swatches from
the same theme instead of per-site colour literals, and a generated style guide
documents the contract.

## ADDED Requirements

### Requirement: Theme token contract

The theme SHALL expose a public accessor that returns the named tokens of a
brightness level: the colour tokens the stylesheet substitutes, including every
derived shade, and the numeric metrics (panel border width, chrome border width,
group divider width, tool-slot footprint and icon maxima, shade step). The
global stylesheet builder SHALL consume the same accessor, so a token value
appears exactly once in the theme and the stylesheet, the dump, and the widgets
cannot disagree. Clamping a level SHALL use the existing level range.

#### Scenario: Tokens cover colours and metrics [ds_tokens_shape]

- **WHEN** the token accessor is asked for any valid brightness level
- **THEN** it returns the named colour tokens (including the derived ones the
  stylesheet uses, such as the alternate table background, menu disabled text,
  panel-header border, and scrollbar thumb hover) and the numeric metrics,
  each exactly once

#### Scenario: The stylesheet and the dump agree [ds_tokens_single_source]

- **WHEN** the stylesheet for a level is built and the same level is dumped
- **THEN** every colour token in the dump appears in the stylesheet with the
  same value, and no colour literal outside the token set is substituted into
  the stylesheet

### Requirement: Machine-readable theme dump

The application SHALL provide a headless `--dump-theme` mode that prints the
theme as JSON on standard output and exits with success without opening a
window or needing a display server. The JSON SHALL cover all brightness levels,
each level carrying its level index, its colour tokens, and its metrics.

#### Scenario: Dump prints all levels as JSON [ds_dump_json]

- **WHEN** the application is run with `--headless --dump-theme`
- **THEN** it prints valid JSON containing every brightness level with its
  colour tokens and metrics, and exits 0 without showing a window

#### Scenario: Dump works without a display [ds_dump_offscreen]

- **WHEN** `--dump-theme` runs with no `DISPLAY` or `WAYLAND_DISPLAY` available
- **THEN** it still succeeds, because the mode selects the offscreen platform

### Requirement: Canonical primitive styles

Each standard primitive SHALL have exactly one global stylesheet definition,
selected by object name where a primitive needs its own surface: text button,
icon/toggle button, text input, numeric field, combo box, check box, slider,
panel tab, document tab, menu, colour swatch, and panel header. Each definition
SHALL cover its interactive states — rest, hover, pressed, checked (where the
primitive toggles), disabled, and keyboard focus (where the primitive accepts
focus) — and every colour SHALL come from the theme tokens.

#### Scenario: Each primitive is defined once [ds_primitive_single_definition]

- **WHEN** the global stylesheet is searched for a primitive's rules
- **THEN** each primitive's states are declared in one place, scoped by type or
  object name, not repeated per call site

#### Scenario: States differ through tokens [ds_primitive_states]

- **WHEN** a button, input, combo, check box, slider, or tab is shown at rest,
  hovered, pressed, checked, disabled, or focused
- **THEN** its state styling resolves to the theme tokens (for example the
  hover surface, the pressed surface, the highlight, and the disabled text),
  and no state falls back to an untokenised literal

### Requirement: No static colour literals in widgets

A widget SHALL NOT carry a per-widget stylesheet whose colours are static
theme concerns; such styling SHALL live in the global stylesheet, scoped by
object name. Colours that are data — a chosen colour, a gradient stop, a
sampled pixel, a preview fill — SHALL keep their dynamic value and SHALL be
rendered through the shared swatch widget rather than through ad-hoc
per-widget stylesheets. Font-weight-only overrides that express a primitive
style (such as a bold panel title or tab) SHALL likewise be expressed through
the shared definition.

#### Scenario: Static colours move to the global stylesheet [ds_no_static_colours]

- **WHEN** a dialog, panel, or option bar builds a standard control
- **THEN** it applies an object name instead of a per-widget stylesheet with a
  hard-coded colour

#### Scenario: Data colours stay data [ds_data_colours]

- **WHEN** a widget displays a colour that is document or user data
- **THEN** the widget still shows that colour, and its styling carries only the
  dynamic fill plus the shared frame treatment

### Requirement: Generated style guide

A repository script SHALL generate `design-system.html` at the repository root
from the `--dump-theme` JSON: the token values become CSS custom properties
that the component markup references, and the page SHALL render every colour
token and metric, every primitive, and every interactive state across all
brightness levels. Running the script again without a theme change SHALL
produce an identical file.

#### Scenario: The guide is generated from the dump [ds_styleguide_generated]

- **WHEN** the generation script runs against a built application
- **THEN** it invokes `--headless --dump-theme`, writes `design-system.html`
  using CSS custom properties derived from the dump, and the page shows the
  tokens, primitives, states, and levels

#### Scenario: Regeneration is idempotent [ds_styleguide_idempotent]

- **WHEN** the script runs twice in a row with no theme change
- **THEN** the second run leaves the file byte-identical

### Requirement: Design-system behavioural check

The Qt Test suite SHALL include a design-system suite that instantiates each
primitive, asserts the object-name and state contract, and asserts that the
colours the palette and stylesheet resolve to equal the values returned by the
token accessor for the applied level.

#### Scenario: Primitives carry their contract [ds_test_primitives]

- **WHEN** the design-system test instantiates a button, icon/toggle button,
  text input, numeric field, combo, check box, slider, tab, menu, swatch, and
  panel header
- **THEN** each instance reports the expected object name and exposes its
  checked/disabled/focus state as the contract defines

#### Scenario: Palette equals tokens [ds_test_tokens]

- **WHEN** the theme is applied at a level and the test reads the widget
  palette and the generated stylesheet
- **THEN** the resolved colours equal the token accessor's values for that
  level
