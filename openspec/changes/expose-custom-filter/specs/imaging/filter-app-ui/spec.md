## ADDED Requirements

### Requirement: Custom dialog

`Filter ▸ Other ▸ Custom…` SHALL be enabled whenever filters are and SHALL sit first in the Other submenu, ahead of High Pass. It SHALL open the filter dialog with a 5×5 grid of integer weight cells, each `-999..=999`, then `Scale:` (`1..=9999`) and `Offset:` (`-9999..=9999`) as value boxes with no slider. The dialog SHALL open on the pass-through kernel: centre cell 1, every other cell 0, Scale 1, Offset 0. The `custom` kind SHALL take 27 slots in order: the 25 cells left to right, top to bottom, then Scale, then Offset; an empty list SHALL be the pass-through kernel and any other length SHALL be refused. Editing a cell or box SHALL preview on the canvas as other filter dialogs do, OK SHALL commit one history state labelled `Filter: Custom`, and `Filter ▸ Last Filter` SHALL re-apply the same kernel.

#### Scenario: The dialog opens on the pass-through kernel

- **WHEN** the Custom dialog opens on a document
- **THEN** it shows 25 cells ranged `-999..=999`, two value boxes and no slider, and its values are 12 zeros, a 1, 12 zeros, then `1, 0`

#### Scenario: The grid reads row by row

- **WHEN** the first cell of the grid's second row is set to -3
- **THEN** slot 5 of the dialog's values is -3

#### Scenario: Inverting through the grid previews, commits, and repeats

- **WHEN** the dialog is opened from the menu, the centre cell is set to -1 and Offset to 255, and OK is pressed
- **THEN** the canvas changes before OK, one `Filter: Custom` history state is added, every channel reads `255 - v`, and `Filter ▸ Last Filter`, labelled `Custom`, inverts the picture back
