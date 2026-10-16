## ADDED Requirements

### Requirement: Filter dialog colour swatches

A colour parameter in a filter dialog SHALL show as a swatch filled with the colour, without the hex code as text. Clicking the swatch SHALL open the application colour picker, and a colour picked there SHALL repaint the swatch and update the preview.

#### Scenario: Neon Glow shows its glow colour as a swatch

- **WHEN** the Neon Glow dialog opens
- **THEN** its Glow Color control is a swatch filled with the default blue and carries no text

### Requirement: Artistic filter dialog defaults

Each Artistic dialog SHALL open on these defaults, and its kind with an empty slot list SHALL map to the same values: Colored Pencil 4 / 8 / 25; Cutout 4 / 4 / 2; Dry Brush and Fresco 2 / 8 / 1; Film Grain 4 / 0 / 10; Neon Glow 5 / 15 with a blue (0, 0, 255) glow; Paint Daubs 8 / 7 / Simple; Palette Knife 25 / 3 / 0; Plastic Wrap 15 / 9 / 7; Poster Edges 2 / 1 / 2; Rough Pastels 6 / 4 / Canvas / 100 % / Relief 20 / Light Bottom; Smudge Stick 2 / 0 / 10; Sponge 2 / 12 / 5; Underpainting 6 / 16 / Canvas / 100 % / Relief 4 / Light Top; Watercolor 9 / 1 / 1.

Colored Pencil, Rough Pastels and Watercolor SHALL show no foreground or background swatches; black and white stand in for the document colours. Rough Pastels and Palette Knife SHALL have no Seed control. Light Direction SHALL be a menu of the eight CS6 directions, placed below Relief with Invert below it, and a dialog of up to eight controls SHALL keep them in one column.

#### Scenario: Dialogs open on their defaults

- **WHEN** the Plastic Wrap, Palette Knife, Colored Pencil, Watercolor, Rough Pastels, and Underpainting dialogs open
- **THEN** their values are 15 / 9 / 7; 25 / 3 / 0; 4 / 8 / 25 / seed; 9 / 1 / 1 / seed; 6 / 4 / Canvas / 100 / 20 / Bottom / off; and 6 / 16 / Canvas / 100 / 4 / Top / off / seed

#### Scenario: Underpainting stays in one column

- **WHEN** the Underpainting dialog opens
- **THEN** every slider sits in the same column

### Requirement: Filter dialogs preview on open

A filter dialog with Preview checked SHALL render its preview on the canvas as soon as it opens, without any control being changed.

#### Scenario: Opening previews

- **WHEN** a filter dialog opens and no control is touched
- **THEN** the canvas shows the filtered preview
