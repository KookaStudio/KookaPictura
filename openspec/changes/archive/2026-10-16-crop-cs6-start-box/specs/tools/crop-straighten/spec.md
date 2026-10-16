## MODIFIED Requirements

### Requirement: Crop preview and active states

The Crop tool SHALL distinguish a preview state (the box the tool starts with,
not yet a pending crop) from an active state. A box in the preview state SHALL
be drawn with its frame and eight handles but without rule-of-thirds guides; an
active box SHALL be drawn with its frame, guides, and eight handles. Selecting
the tool in either mode SHALL show a preview box fitted to the active aspect
ratio and centered on the canvas, as CS6 frames the whole image. Dragging a
preview handle SHALL resize the box, which becomes the active crop box; dragging
inside the preview SHALL draw a new active box clamped to the canvas; a press and release without movement inside the
preview SHALL adopt the preview as the active box; a rotate press outside SHALL
activate the box and begin rotation. Escape and Cancel SHALL clear the box and
return to no box in both modes.

#### Scenario: Modern starts in preview [lcs_preview_modern]

- **WHEN** the Crop tool is selected in Modern mode
- **THEN** a preview box, centered and fitted to the active ratio, is shown with its eight handles and no guides

#### Scenario: Classic starts in preview too [lcs_preview_classic]

- **WHEN** the Crop tool is selected in Classic mode
- **THEN** the same preview box with its handles is shown, not an empty canvas

#### Scenario: A preview handle resizes [lcs_preview_handle]

- **WHEN** a corner or edge handle of the preview box is dragged
- **THEN** the box is resized from that handle and becomes the active crop box

#### Scenario: Dragging the preview draws a new box [lcs_preview_draw]

- **WHEN** a drag begins inside the preview box
- **THEN** a new active crop box is drawn, clamped to the canvas

#### Scenario: Clicking the preview activates it [lcs_preview_click]

- **WHEN** the preview box is pressed and released without movement
- **THEN** the preview box becomes the active crop box

#### Scenario: Escape clears to no box [lcs_preview_escape]

- **WHEN** Escape or Cancel is invoked in either mode
- **THEN** the box and angle are cleared and no box is shown

### Requirement: Straighten from the init state

A straighten line drawn while no active box exists SHALL activate the crop first
so that the rotation can be previewed: the preview box when one is shown, or the
full canvas when there is no box (after Escape). The rotation SHALL preview from the
first drag frame and update immediately on release.

#### Scenario: Drawing a straighten line activates the crop [lcs_straighten_activates]

- **WHEN** the straighten toggle is armed and a horizon line is drawn with no active box
- **THEN** a crop box is activated and the content rotation previews during the drag and on release
