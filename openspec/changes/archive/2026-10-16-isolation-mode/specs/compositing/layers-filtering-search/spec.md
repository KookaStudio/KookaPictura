# layers-filtering-search Specification (delta)

## MODIFIED Requirements

### Requirement: Filter row and controls

The Layers panel SHALL provide a filter/search row at the top of the header,
containing a dimension popup, a criteria control, and an on/off lightswitch
toggle. The dimension popup SHALL offer Name, Kind, Effect, Mode, Attribute,
Color, and Selected, defaulting to Kind. Selecting a dimension SHALL swap the
criteria control to that dimension's editor: Name is a free-text field, Kind is
a set of toggle buttons (multi-select), Effect, Mode, Attribute, and Color are
value menus, and Selected has no criteria control because its criterion is the
isolated set. Each Kind toggle button SHALL carry an icon for its kind —
pixel, adjustment, group, background, or type — with the kind name as its
tooltip. The Effect dimension SHALL be present but disabled, with a "not
implemented yet" tooltip, until layer effects exist; the other six dimensions
SHALL work against the data the row projection already exposes. The toggle SHALL
be shown as a lightswitch icon, SHALL default to on, and SHALL enable or disable
the predicate without discarding the chosen dimension or criteria. With no
criterion chosen the enabled toggle SHALL leave every row visible.

#### Scenario: The Kind buttons carry icons [lfs_row]

- **WHEN** the Kind dimension is shown
- **THEN** each of its toggle buttons displays a kind icon and its tooltip is
  the kind name

#### Scenario: The row defaults to the Kind dimension with the toggle on [lfs_row]

- **WHEN** the Layers panel is shown
- **THEN** the filter/search row is present, its dimension popup reads Kind, and
  the lightswitch toggle is on

#### Scenario: The toggle is a lightswitch icon [lfs_row]

- **WHEN** the filter/search row is shown
- **THEN** the on/off control is an icon button (a lightswitch) rather than a
  text button, and its icon reflects the on/off state

#### Scenario: Choosing a dimension swaps the criteria control [lfs_row]

- **WHEN** the user picks Name from the dimension popup
- **THEN** the criteria control becomes a text field, and picking Mode makes it
  a blend-mode menu

#### Scenario: Selected has no criteria control [lfs_row_selected]

- **WHEN** the user picks Selected from the dimension popup
- **THEN** the criteria control becomes empty, because the criterion is the
  current selection

#### Scenario: The Effect dimension is disabled [lfs_row]

- **WHEN** the user opens the dimension popup before layer effects exist
- **THEN** the Effect entry is disabled and selecting it leaves the filter
  inactive

#### Scenario: An on-by-default toggle with no criterion shows every row [lfs_row]

- **WHEN** the panel is shown with the lightswitch on and no criterion chosen
- **THEN** every layer row is visible

### Requirement: Filter predicate

The system SHALL match a layer against the active filter as follows. Name SHALL
match as a case-insensitive substring of the layer or group name. Kind SHALL
match when the layer's kind is one of the selected kinds, where the selectable
kinds are exactly those the model exposes (pixel, adjustment, group,
background, type). Mode SHALL match the layer's blend-mode key exactly. Color
SHALL match the layer's color label index exactly. Attribute SHALL match one of
Visible, Hidden, Locked, Has Mask, or Clipped against the row's state. Selected
SHALL match when the row's layer is in the isolated set. A
group ancestor of a matching descendant SHALL be shown so the hierarchy stays
navigable. Multiple active criteria SHALL be combined with AND, and the selected
Kind values with OR. A dimension with no value selected or typed SHALL be
inactive. A filter that matches no layer SHALL show an empty list without
changing the document.

#### Scenario: Name narrows to a substring [lfs_name]

- **WHEN** the Name filter is enabled with `sky`
- **THEN** only layers and groups whose names contain `sky` (case-insensitively)
  are shown, and every other layer is hidden from the panel

#### Scenario: Kind is a multi-select [lfs_kind]

- **WHEN** the Kind filter has pixel and adjustment selected
- **THEN** pixel and adjustment layers are shown and group and background layers
  are hidden

#### Scenario: Type kind is selectable [lfs_kind]

- **WHEN** the Kind filter has only type selected
- **THEN** type layers are shown and pixel, adjustment, group, and background
  layers are hidden

#### Scenario: Mode matches the blend key [lfs_mode]

- **WHEN** the Mode filter is enabled with Multiply
- **THEN** only layers whose blend mode is Multiply are shown

#### Scenario: Color matches the label [lfs_color]

- **WHEN** the Color filter is enabled with Red
- **THEN** only layers whose color label is Red are shown

#### Scenario: Selected matches the isolated set [lfs_selected]

- **WHEN** the Selected filter is enabled with an isolated set of two layers
- **THEN** only those two layers (and their ancestor groups) are shown

#### Scenario: Criteria combine with AND [lfs_mode]

- **WHEN** Name `shadow` and Mode Multiply are both active
- **THEN** only layers matching both are shown

#### Scenario: No match leaves the document unchanged [lfs_none]

- **WHEN** the filter matches no layer
- **THEN** the panel is empty, the document's layers and visibility are
  unchanged, and toggling the filter off restores every row
