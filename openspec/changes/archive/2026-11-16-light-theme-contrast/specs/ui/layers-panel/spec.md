# Spec Delta — ui/layers-panel

## MODIFIED Requirements

### Requirement: Layer row surface and two-column layout

Each layer row's background SHALL be one shade step toward the family's
contrasting direction from the list/base background — lighter on the dark family
and darker on the light family. Each layer row SHALL be split into two columns —
a fixed visibility-toggle column and a content column holding the rest of the
row — so a click in the visibility column toggles visibility and never selects
or highlights the row. A synthetic effect or filter row SHALL sit inside the
layer's content column: the group header's eye aligns with the owning layer's
image-thumbnail right edge and each child row sits one indent step deeper, and
the fixed left column continues as the layer's spanning left cell, which is not
a toggle target for that synthetic row. The selected-row highlight SHALL be a
neutral grey stepped further in the same family direction than the row surface,
painted directly by the row delegate rather than the palette's blue highlight,
and the delegate SHALL be the single source of that selection colour with no
scoped stylesheet selection rule and no style-drawn focus tint for the tree
view; a focused selected row SHALL match an unfocused one. The content column
SHALL carry a 2 px left padding so the thumbnail and name are not flush against
the visibility gutter; a synthetic row's content begins after its aligned eye.
The visibility column's flat fill SHALL take the row's selection surface when the
row is selected, and its 1 px gutter separator SHALL remain unchanged by
selection; the gutter separator SHALL stay 1 px. The selected name SHALL use the
palette's highlighted text on that neutral selection surface, so it stays
legible in either family.

#### Scenario: The row background steps lighter than the list [lpr_row_surface]

- **WHEN** a layer row is shown against the list background in either family
- **THEN** the row background is one shade step toward the family's contrasting
  direction from the list background

#### Scenario: The visibility toggle cannot be highlighted [lpr_eye_no_select]

- **WHEN** a click lands in a layer row's visibility-toggle column
- **THEN** visibility toggles and the row is neither selected nor highlighted

#### Scenario: Selection uses a lighter grey [lpr_selection_grey]

- **WHEN** one or more layer rows are selected
- **THEN** the row delegate paints the selected-row background as a neutral grey
  stepped further in the family direction than the row, rather than the blue
  highlight, with no separate stylesheet rule or focus tint overriding it

#### Scenario: A focused selected row keeps the flat highlight [lpr_selection_focus]

- **WHEN** a selected layer row's tree gains focus and then loses it
- **THEN** the row uses the same flat grey highlight in both states, with no
  focus tint or gradient

#### Scenario: The content column has a 2 px left padding [lpr_content_padding]

- **WHEN** a layer row is shown
- **THEN** the content column begins 2 px to the right of the visibility gutter,
  with the gutter separator still 1 px

#### Scenario: Selected row text stays legible in the light family [lpr_light_row_contrast]

- **WHEN** the light family is applied and a layer row is selected
- **THEN** the selection surface is darker than the list background rather than
  white, and the selected name's highlighted text paints visibly on it
