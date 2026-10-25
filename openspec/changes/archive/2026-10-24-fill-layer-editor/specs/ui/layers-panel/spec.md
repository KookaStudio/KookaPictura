# Spec Delta — ui/layers-panel

## MODIFIED Requirements

### Requirement: Fill / Adjustment creation menu

The Layers panel's New Fill / Adjustment strip button SHALL open a menu whose
Fill group offers Solid Color…, Gradient…, and Pattern…. Solid Color and
Gradient SHALL create the matching fill layer through the bridge. Pattern SHALL
be enabled when the open document contains at least one pattern resource and
SHALL be disabled with a "No patterns are loaded" reason when it contains none.
The Adjustment group SHALL list all sixteen CS6 adjustment kinds. Choosing an
adjustment kind SHALL create that adjustment layer through the bridge. The Fill
and Adjustment groups SHALL be separated as in CS6. The `Layer > New Fill Layer`
submenu SHALL offer the same three fill types with the same Pattern gating.

#### Scenario: The menu lists every adjustment kind

- **WHEN** the New Fill / Adjustment menu is opened
- **THEN** it contains an entry for each of the sixteen adjustment kinds, each enabled, and the choosing creates the matching layer

#### Scenario: Pattern stays disabled until authoring exists

- **WHEN** the New Fill / Adjustment menu is opened on a document with no pattern resources
- **THEN** the Pattern… entry is present, disabled, and carries a no-patterns reason

#### Scenario: Pattern follows the document's pattern resources [lpr_pattern_entry]

- **WHEN** the menu is opened on a document with patterns and on one without
- **THEN** Pattern… is enabled on the first and disabled with the no-patterns reason on the second

#### Scenario: The Layer menu offers the fill types [lpr_layer_fill_menu]

- **WHEN** the `Layer > New Fill Layer` submenu is built
- **THEN** it offers Solid Color…, Gradient…, and Pattern… with the same enablement as the panel menu

### Requirement: Layer Content Options opens the fill / adjustment editor

`Layer > Layer Content Options…` SHALL be enabled only when the current layer is
a fill or adjustment layer. Invoking it on a fill layer SHALL open that fill
type's editor; invoking it on an adjustment layer SHALL show the Properties
panel and refresh it so it reflects that layer. For any other current layer the
command SHALL be disabled and invoking it SHALL do nothing.

#### Scenario: The command opens the fill editor

- **WHEN** a Solid Color, Gradient, or Pattern fill layer is current and the command is invoked
- **THEN** the matching fill editor opens on that layer

#### Scenario: The command opens Properties for an adjustment layer

- **WHEN** an adjustment layer is current and `Layer Content Options…` is invoked
- **THEN** the Properties panel is shown and reflects that layer

#### Scenario: The command is disabled for other layers

- **WHEN** the current layer is a pixel, type, group, or Background layer
- **THEN** `Layer Content Options…` is disabled and invoking it does nothing

### Requirement: Layer style panel integration

The Layers panel SHALL be the surface that reaches the layer-style feature. A
row double-click outside the name, the eye/mask controls, and a fill or
adjustment layer's content thumbnail SHALL open the Layer Style dialog on
Blending Options for that layer when the layer can carry a style
(`layer_style_can_edit`), and SHALL do nothing otherwise. A double-click on an
adjustment layer's content thumbnail SHALL show the Properties panel; a
double-click on a fill layer's content thumbnail SHALL open its fill editor. The
row context menu SHALL offer **Blending Options…**, **Copy Layer Style**,
**Paste Layer Style**, and **Clear Layer Style** as implemented commands:
Blending Options SHALL appear for pixel, group, and type rows and SHALL be
enabled only where the layer can carry a style; Copy and Clear SHALL be enabled
when the row's layer carries a style; Paste SHALL be enabled when the app-wide
style clipboard is non-empty. Choosing a command SHALL act on the row's layer
through the layer-style bridge and SHALL record through the bridge's own
history. The Background's brief row menu SHALL NOT offer Blending Options or the
style clipboard rows.

An `Alt`-click on a styled-or-adjustment row's right-edge `fx` region SHALL
toggle every layer's effects through `layer_style_set_all_visible`, showing
them when all were hidden and hiding them when any were shown, as one undoable
step.

The filter row's **Effect** dimension SHALL be enabled and SHALL list the ten
effect keys from `layer_style_effect_names()`; selecting an effect SHALL match
the rows whose layer carries that effect (and, as with the other dimensions,
their ancestors), and a row whose layer does not carry it SHALL be filtered out.

#### Scenario: A double-click opens Blending Options

- **WHEN** a pixel layer row is double-clicked outside its name and its eye/mask controls
- **THEN** the Layer Style dialog opens on Blending Options for that layer

#### Scenario: A background double-click does not open a style

- **WHEN** the Background row is double-clicked outside its name
- **THEN** the Background conversion path runs and no Layer Style dialog opens

#### Scenario: The style rows report their enablement

- **WHEN** the row menu is opened for a pixel layer that carries a color overlay and has a copied style available
- **THEN** Blending Options, Copy Layer Style, Paste Layer Style, and Clear Layer Style are all enabled

#### Scenario: An unstyled pixel row disables Copy and Clear

- **WHEN** the row menu is opened for a pixel layer with no layer style
- **THEN** Blending Options is enabled and Copy Layer Style, Paste Layer Style, and Clear Layer Style are disabled

#### Scenario: An adjustment row omits the style rows

- **WHEN** the row menu is opened for an adjustment layer
- **THEN** it does not offer Blending Options, Copy Layer Style, Paste Layer Style, or Clear Layer Style

#### Scenario: The row menu clears a row's style

- **WHEN** Clear Layer Style is chosen for a pixel row that carries a style
- **THEN** the row's style is removed in one undoable step and its fx badge disappears

#### Scenario: Alt-clicking the fx region toggles all effects

- **WHEN** the user `Alt`-clicks the fx badge of a styled row while effects are shown
- **THEN** every layer's effects are hidden in one undoable step, and a second `Alt`-click shows them again

#### Scenario: The Effect dimension matches styled rows

- **WHEN** the filter uses the Effect dimension with an effect a layer carries
- **THEN** that layer's row is shown and rows without the effect are hidden

#### Scenario: The Effect dimension lists the effect keys

- **WHEN** the filter's Effect dimension is selected
- **THEN** its choices are the ten effect keys from the layer-style effect list

#### Scenario: An adjustment content thumbnail opens Properties [lpr_adjustment_thumb_opens]

- **WHEN** the user double-clicks an adjustment layer's content thumbnail
- **THEN** the Properties panel is shown on that adjustment and no Layer Style dialog opens

#### Scenario: A fill content thumbnail opens the fill editor [lpr_fill_thumb_opens]

- **WHEN** the user double-clicks a fill layer's content thumbnail
- **THEN** the matching fill editor opens and no Layer Style dialog opens
