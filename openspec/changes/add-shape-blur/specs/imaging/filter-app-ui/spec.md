## ADDED Requirements

### Requirement: Shape Blur dialog

`Filter ▸ Blur ▸ Shape Blur…` SHALL be enabled whenever filters are and SHALL sit after Box Blur in the Blur submenu. It SHALL open the filter dialog with `Radius:` (`5..=1000` pixels, default 10) and a `Shape:` grid holding every Custom Shape in the catalogue's order, each with its name as a tooltip, the first selected by default. The `shape-blur` kind SHALL take 2 slots in order: Radius, then the shape's index; an empty list SHALL be radius 10 with the first shape, an index past the catalogue SHALL clamp to the last shape, and any other length SHALL be refused. Changing the radius or the shape SHALL preview on the canvas as other filter dialogs do, OK SHALL commit one history state labelled `Filter: Shape Blur`, and `Filter ▸ Last Filter` SHALL re-apply the same shape and radius.

#### Scenario: The dialog opens on radius 10 and the first shape

- **WHEN** the Shape Blur dialog opens on a document
- **THEN** its values are `10, 0`, the radius box ranges `5..=1000`, and the grid lists every Custom Shape with the Heart second

#### Scenario: Picking a shape fills the second slot

- **WHEN** the third shape in the grid is selected
- **THEN** slot 1 of the dialog's values is 2

#### Scenario: Picking previews, commits, and repeats

- **WHEN** the dialog is opened from the menu, the Heart and radius 6 are chosen, and OK is pressed
- **THEN** the canvas changes before OK, one `Filter: Shape Blur` history state is added, and `Filter ▸ Last Filter`, labelled `Shape Blur`, blurs the picture again
