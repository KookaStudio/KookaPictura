# Spec Delta — ui/layers-panel

## MODIFIED Requirements

### Requirement: Inline rename with Tab navigation

The system SHALL begin inline name editing only when the user double-clicks
within the row's **rendered name label**: the painted name text's width after
right-elision to the available label band, floored to a one-pixel target at the
label's left edge. It SHALL NOT begin editing on the visibility, chevron,
thumbnail, lock, fx, or mask regions, nor on empty row space to the right of the
rendered label. The hit rectangle SHALL be the `nameRect` geometry the delegate
paints, including the name gap after the thumbnail block and the right-edge
badge/mask caps, so paint and hit-testing cannot drift. A double-click outside
the rendered label but outside those controls SHALL invoke the row's layer-style
affordance instead of beginning an edit. When the double-clicked row is the
Background layer, the system SHALL convert it to a normal layer (through the
name-and-color dialog) instead of renaming, wherever in the row outside the
controls it lands; for every other layer it SHALL invoke the layer-style
affordance, which is a documented no-op when the layer cannot carry a style.
While editing, the system SHALL commit the edit and move to the next visible row
when the user presses `Tab`, and to the previous visible row when the user
presses `Shift+Tab`. At the last or first visible row, the commit SHALL occur
without wrapping.

#### Scenario: Double-click starts editing [m39_rename]

- **WHEN** the user double-clicks a row's rendered name label
- **THEN** an editor opens on that row and the document is unchanged until the
  edit commits

#### Scenario: Double-click anywhere in the label band renames [lpr_rename_band]

- **WHEN** the user double-clicks a non-Background row inside its rendered name
  label, including a row whose thumbnails are off
- **THEN** the inline editor opens for that row

#### Scenario: Double-click outside the rendered label opens Layer Style [lpr_rename_outside_label]

- **WHEN** the user double-clicks a styled non-Background row to the right of its
  rendered name label and outside the eye/chevron/thumbnail/lock/fx/mask controls
- **THEN** no inline editor opens and the Layer Style dialog opens on Blending
  Options for that layer

#### Scenario: Double-click on a control does not rename [lpr_rename_name_only]

- **WHEN** the user double-clicks the eye, chevron, thumbnail, lock, fx, or mask
  control
- **THEN** no inline editor opens and the control's own action runs

#### Scenario: Double-clicking the Background converts it [lpr_background_dblclick]

- **WHEN** the user double-clicks the Background row outside the
  eye/chevron/thumbnail/lock controls
- **THEN** the Background is converted through the name-and-color dialog rather
  than opening an inline editor

#### Scenario: Tab commits and moves down [m39_rename]

- **WHEN** the user edits a name and presses `Tab`
- **THEN** the name changes, the edit is one undo state, and the editor moves to
  the next visible row

#### Scenario: Shift+Tab moves up without wrapping [m39_rename]

- **WHEN** the user is editing the first visible row and presses `Shift+Tab`
- **THEN** the edit commits and editing ends without wrapping to the last row
