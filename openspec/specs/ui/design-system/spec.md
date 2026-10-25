# ui/design-system Specification

## Purpose
One token contract and one canonical definition per standard control, so every
dialog, panel, and option bar draws its buttons, fields, tabs, and swatches from
the same theme instead of per-site colour literals, and a generated style guide
documents the contract.

## Requirements

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
rendered through the sh literals in widgets

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

### Requirement: Light theme family

The theme SHALL provide a light family in addition to the dark family. Both
families SHALL share the same token keys, and the light family's values SHALL
mirror the dark family role-for-role: light surfaces (window, panel, workspace,
headers, inputs, buttons) and dark text, icons, and borders. The light family
SHALL expose the same number of brightness levels as the dark family. Switching
family SHALL re-apply the palette and the stylesheet and SHALL invalidate the
icon caches so icons and custom-painted widgets follow. The chosen family SHALL
persist across restarts, and a session with no stored family SHALL load the
dark family. The dark family's default-level values SHALL be unchanged. Derived
shades — the alternate table background, menu disabled text, panel-header
border, scrollbar thumb hover, and separator light — SHALL stay distinct from
their base colour in each family, deriving toward that family's shaded
direction rather than clamping together.

#### Scenario: Light roles mirror the dark roles [ds_light_roles_mirror]

- **WHEN** the light family is applied at its default level
- **THEN** every role key resolves to a value, each light surface token is
  lighter than its matching text/icon token, and the dark family's default-level
  colour tokens are unchanged

#### Scenario: Derived shades stay distinct [ds_light_derived_distinct]

- **WHEN** the derived shade tokens are resolved in either family
- **THEN** each derived shade differs from its base token, so alternate table
  rows, disabled menu text, and hover outlines remain distinguishable

#### Scenario: Switching family persists and follows into paint [ds_light_switch_persists]

- **WHEN** the user switches the theme family and restarts the application
- **THEN** the chosen family is restored, and icons and custom-painted widgets
  render with the restored family's colours

### Requirement: Menu-bar chrome actions

The application SHALL show a theme toggle and GitHub and Discord link buttons at
the top-right of the menu-bar row. The theme button SHALL show a sun while the
dark family is active and a moon while the light family is active, and clicking
it SHALL switch the family. The link buttons SHALL open the project's Discord
invite and GitHub repository in the user's browser. All three SHALL draw from
the theme tokens.

#### Scenario: The theme button reflects and switches the family [ds_chrome_theme]

- **WHEN** the dark family is active
- **THEN** the theme button shows a sun; clicking it switches to the light
  family, and the button then shows a moon

#### Scenario: The links open the project pages [ds_chrome_links]

- **WHEN** the Discord or GitHub button is clicked
- **THEN** the project's Discord invite or GitHub repository URL opens in the
  default browser

### Requirement: Remaining standard controls

Each remaining standard control SHALL have exactly one global stylesheet
definition, selected by object name where a control needs its own surface:
group box, radio button, progress bar, header view, and splitter. Each
definition SHALL cover its interactive states — rest, hover, pressed, checked
(where the control toggles), disabled, and keyboard focus (where the control
accepts focus) — and every colour SHALL come from the theme tokens. Controls
already tokenised SHALL keep their existing single definition.

#### Scenario: Each remaining control is defined once [ds_remaining_single_definition]

- **WHEN** the global stylesheet is searched for a remaining control's rules
- **THEN** each control's states are declared in one place, scoped by type or
  object name, not per call site

#### Scenario: Remaining controls resolve through tokens [ds_remaining_states]

- **WHEN** a group box, radio button, progress bar, header view, or splitter is
  shown at rest, hovered, pressed, checked, disabled, or focused
- **THEN** its state styling resolves to theme tokens and no state falls back to
  an untokenised literal

### Requirement: Custom-painted widgets read tokens

A widget that paints itself SHALL derive every theme-concern colour from the
theme tokens (directly, or through the widget palette the token layer applies)
and SHALL NOT hard-code a theme colour in its paint code. Colours that are data
MAY keep a dynamic value. The custom-painted widgets that still hard-code theme
greys — the curve editor, the spectrum bar, the histogram view, and the
navigator thumbnail — SHALL be converted; widgets that already derive from the
palette are unchanged.

#### Scenario: Custom-painted widgets follow the theme [ds_painted_tokens]

- **WHEN** the curve editor, spectrum bar, histogram view, and navigator
  thumbnail repaint after a theme change
- **THEN** every theme-concern colour they draw equals a value from the token
  accessor, and no hard-coded theme grey remains

#### Scenario: Data colours stay data [ds_painted_data]

- **WHEN** a custom-painted widget draws a colour that is document or user data
- **THEN** it keeps that dynamic value and only its frame or background comes
  from the tokens

### Requirement: Theme-like colour literals are rejected

The repository SHALL provide a check that fails when a widget source outside the
theme layer introduces a theme-like grey colour literal — a hard-coded grey
surface, text, border, or control colour. The check SHALL ship with a baseline
allow-list capturing the pre-existing grep hits so it passes on landing: genuine
data-colour sites (the canvas backdrop, marching-ants pairs, greyscale
gradients, the foreground/background tool swatches, liquify and guide presets)
and the theme-like literals still pending conversion, each recorded with a
reason. The list only shrinks as conversion proceeds; a new literal not on it
fails.

#### Scenario: The guard rejects a new theme literal [ds_literal_guard_rejects]

- **WHEN** a source file outside the theme layer introduces a theme-like grey
  literal that is not allow-listed
- **THEN** the check fails and names the offending file

#### Scenario: Allow-listed data colours pass [ds_literal_guard_allows]

- **WHEN** an allow-listed data-colour site keeps its literal
- **THEN** the check passes

#### Scenario: The baseline is green on landing [ds_literal_guard_baseline]

- **WHEN** the check runs over the tree as landed
- **THEN** it passes, because every pre-existing hit is either converted or
  recorded on the baseline allow-list, and the list may only be pruned

### Requirement: In-app design-system browser

The application SHALL provide an in-app design-system browser reachable from
Help ▸ Developer ▸ Design System…, shown as a modeless tool window. The browser
SHALL present the design system using the application's own widgets and the
global stylesheet rather than a reimplementation, grouped into foundations,
shell, controls, and patterns, with a filterable story list, a per-story
description, and a token readout for the current brightness level. Every
primitive the global stylesheet defines SHALL appear on some story.

#### Scenario: The browser opens modelessly [ds_browser_modeless]

- **WHEN** the user chooses Help ▸ Developer ▸ Design System…
- **THEN** a modeless tool window opens on the grouped story list with a story
  shown, and the rest of the application remains interactive

#### Scenario: Stories render the real widgets [ds_browser_real_widgets]

- **WHEN** a story is selected
- **THEN** its page is built from the application's own widgets, styled by the
  global stylesheet, so it matches the running interface exactly

#### Scenario: Every primitive is covered [ds_browser_coverage]

- **WHEN** the global stylesheet declares a primitive
- **THEN** the browser renders an instance of it on some story

#### Scenario: Brightness preview is live and restored [ds_browser_brightness]

- **WHEN** the brightness control is changed
- **THEN** the application recolours to that level immediately; and when the
  browser is closed, the level in effect when it opened is restored, so
  previewing never changes the user's brightness preference

#### Scenario: The story list is filterable [ds_browser_filter]

- **WHEN** text is entered in the filter field
- **THEN** only the stories whose titles match remain visible

#### Scenario: The browser is covered by a Qt test [ds_test_browser]

- **WHEN** the design-system test opens the browser and builds every story
- **THEN** every story page builds without error, each curated primitive is
  present, the filter narrows the list, and closing restores the brightness
  level

### Requirement: Inset slider-popup arrow

The numeric field's slider-popup arrow SHALL be drawn inside the value input, not
as a sibling outside its frame. The arrow SHALL be a child of the value text
editor, placed flush to its inner trailing edge and sized to the editor's inner
height, so the value box, its hairline separator, and the arrow read as one
control. Opening the popup SHALL mark the arrow active (a darker surface) and
closing it SHALL return the arrow to rest. The field's reserved text width SHALL
account for the arrow so the value never runs under it.

#### Scenario: The arrow is part of the input [ds_arrow_inset]

- **WHEN** a numeric field with a slider popup is laid out
- **THEN** the arrow's parent is the value text editor and the arrow's geometry
  lies inside the editor's rect, with the editor's trailing text margin at least
  the arrow's width

#### Scenario: The arrow is active while the popup is open [ds_arrow_active]

- **WHEN** the popup is opened from the arrow and then hidden
- **THEN** the arrow is active (styled darker) while the popup is shown and at
  rest once it is hidden

### Requirement: Input-family consistency

Every text input, editable input, and select SHALL share one surface definition:
the same background token, the same border token, the same focus treatment, the
same disabled treatment, and the same rendered height, declared once in the
global stylesheet. The numeric field, the editable combo box's inner editor, and
the options-bar workspace switcher SHALL resolve to that same definition. Input, editable-input, and select
text SHALL use a single shared size one pixel under the application default, so
no input family member disagrees. The down/up arrow SHALL be a single Lucide
chevron. The numeric field's arrow and spin-box arrows SHALL be tinted at render
time; the select's drop-down chevron uses the same glyph in the family's
foreground colour, because the stylesheet engine cannot tint an SVG and the
foreground is constant across the brightness levels. The select's drop-down cell
SHALL be flat and SHALL reserve the value's trailing space so the chevron never
overlaps the value. Spin-box arrows SHALL use the same chevron.

#### Scenario: One input surface [ds_input_family_surface]

- **WHEN** a line edit, spin box, combo box, or editable combo box is shown at
  rest, hovered, focused, or disabled
- **THEN** its state styling resolves to the shared input tokens, and the
  editable combo's inner editor adds no second frame

#### Scenario: One shared input text size [ds_input_family_font]

- **WHEN** the global stylesheet is built
- **THEN** every input, editable input, and select rule uses the same font size,
  one pixel under the application default

#### Scenario: One shared input height [ds_input_family_height]

- **WHEN** a line edit, combo box, and spin box are shown together
- **THEN** they render at the same height, the spin box's button floor carried to
  the other inputs

#### Scenario: The select and workspace switcher match [ds_input_family_select]

- **WHEN** a combo box and the workspace switcher are shown
- **THEN** both draw the shared input surface, a flat drop-down cell that reserves
  the value's trailing space, and the same Lucide chevron in the family
  foreground colour

#### Scenario: The chevron follows the family [ds_input_family_chevron]

- **WHEN** the theme family changes
- **THEN** every select and spin-box arrow is drawn in the new family's
  foreground colour, up and down pointing as the control requires

### Requirement: Checkbox tick

A checked checkbox SHALL show a tick mark inside the box, not a bare fill. The
tick SHALL be drawn in the checked box's legible foreground colour and SHALL
remain visible in the checked hover and disabled states.

#### Scenario: The checked box shows a tick [ds_checkbox_tick]

- **WHEN** a checkbox is checked
- **THEN** the indicator draws a tick mark over the highlight fill

### Requirement: Menu popup chrome
Menu popups SHALL paint an outline with a corner radius of at least 3 CSS pixels
at 100 percent UI scale and SHALL be translucent outside that outline. Their
items SHALL be inset from the outline by 2 CSS pixels at 100 percent scale. An
item that carries an icon or check mark SHALL leave at least 8 CSS pixels
between that glyph and the popup's left edge at 100 percent scale.

#### Scenario: Rounded popup corners [ds_menu_rounded]
- **WHEN** a menu is shown
- **THEN** the popup is transparent outside the rounded outline and a selected top row does not paint across the corner

#### Scenario: Icon inset [ds_menu_icon_padding]
- **WHEN** a menu item carries an icon or check mark
- **THEN** the glyph's left edge sits at least 8 CSS pixels from the popup's left edge at 100 percent scale
