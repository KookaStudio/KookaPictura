## ADDED Requirements

### Requirement: Layer effects disclosure rows

The Layers panel SHALL list a styled layer's effects nested under its row: one
synthetic **Effects** group row at the layer's depth plus one, and one synthetic
child row per effect the layer currently carries at the layer's depth plus two,
named with the effect's CS6 title in the layer-style list order (Bevel & Emboss
through Drop Shadow). The layer row SHALL expose the group through its `fx`
badge: clicking the badge SHALL expand or collapse the group. The **Effects**
group row SHALL carry a disclosure that expands or collapses the effect rows and
an eye that toggles the layer's master effect switch (`fx.visible`); each effect
row SHALL carry an eye that toggles that effect's enabled state
(`<effect>.on`), while the effect stays listed so it can be switched back on. A
newly styled layer SHALL start expanded when the panel option **Expand New
Effects** is on, and a layer with no effects SHALL show no group row. Each eye
toggle SHALL be one undoable history state. A synthetic effects row SHALL NOT be
renamed, dragged, dropped on, or made the document's layer target.

#### Scenario: A styled layer shows its effects nested [lpr_fx_rows]

- **WHEN** a pixel layer carries a drop shadow and is shown in the panel
- **THEN** its row is followed by a synthetic `Effects` group row and a `Drop
  Shadow` effect row at the next two depths

#### Scenario: The Effects eye toggles the master switch [lpr_fx_group_eye]

- **WHEN** the `Effects` group row's eye is clicked while the layer's effects are shown
- **THEN** `fx.visible` becomes false, the composite shows the layer unstyled,
  and the row's eye reads hidden

#### Scenario: An effect eye toggles that effect [lpr_fx_effect_eye]

- **WHEN** a `Drop Shadow` effect row's eye is clicked
- **THEN** `dropShadow.on` flips and the composite changes, while the effect row
  stays listed

#### Scenario: A new style starts expanded [lpr_fx_expand_new]

- **WHEN** a layer with no style gains one while **Expand New Effects** is on
- **THEN** the layer's `Effects` group and its effect rows are shown expanded

#### Scenario: An unstyled layer shows no group [lpr_fx_no_group]

- **WHEN** a layer that carries no layer style is shown
- **THEN** no `Effects` row follows it

### Requirement: Layer effects row interactions

Double-clicking a layer's `fx` badge or its **Effects** group row SHALL open the
Layer Style dialog on Blending Options for that layer; double-clicking an effect
row SHALL open it on that effect's page. Right-clicking the `fx` badge, the
**Effects** group row, or an effect row SHALL open the CS6 layer-style menu,
which SHALL offer **Disable Layer Effects** (or **Enable Layer Effects** when the
layer's effects are hidden), **Blending Options…**, the ten effect entries, the
**Copy** / **Paste** / **Clear Layer Style** commands, **Global Light…** and
**Create Layer** disabled, **Hide All Effects** (or **Show All Effects**), and
**Scale Effects…**. **Disable/Enable Layer Effects** SHALL write the layer's
`fx.visible` as one undoable state. Each effect entry SHALL open the Layer Style
dialog on that effect's page, and the menu SHALL act on the layer that owns the
effects.

#### Scenario: Double-clicking an effect row opens its page [lpr_fx_dblclick_effect]

- **WHEN** the user double-clicks a `Drop Shadow` effect row
- **THEN** the Layer Style dialog opens on the Drop Shadow page for that layer

#### Scenario: Double-clicking the fx badge opens Blending Options [lpr_fx_dblclick_badge]

- **WHEN** the user double-clicks a styled layer's `fx` badge
- **THEN** the Layer Style dialog opens on Blending Options for that layer

#### Scenario: The layer-style menu lists the CS6 commands [lpr_fx_menu]

- **WHEN** the user right-clicks the `fx` badge or an effects row of a styled layer
- **THEN** the menu offers Disable Layer Effects, Blending Options…, the ten
  effect entries, Copy/Paste/Clear Layer Style, Global Light… and Create Layer
  disabled, Hide All Effects, and Scale Effects…

#### Scenario: Disable Layer Effects hides the layer's effects [lpr_fx_disable]

- **WHEN** Disable Layer Effects is chosen for a styled layer
- **THEN** the layer's `fx.visible` becomes false, its composite is unstyled, and
  exactly one history state is recorded
