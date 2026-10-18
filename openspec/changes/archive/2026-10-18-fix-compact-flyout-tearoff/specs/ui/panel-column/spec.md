# Spec Delta

## ADDED Requirements

### Requirement: Flyout drag handoff

Dragging out of the compact-strip flyout SHALL hand the hosted `PanelGroup` back
to the drag system without leaving the popup behind as an empty shell. Dragging
the group out of the flyout SHALL close the flyout before the floating overlay
follows the cursor, and the overlay SHALL keep the grab offset the group had in
the popup. Dragging a single tab out of the flyout SHALL keep the flyout open on
the remaining visible panels; when the drag leaves no visible panel, the flyout
SHALL close instead. The existing tear-off, re-dock, cancel, and drop rules SHALL
apply unchanged to the drag.

#### Scenario: A group dragged out of the flyout closes it [cfd_group_tearoff]

- **WHEN** a group is dragged out of an open compact-strip flyout and dropped
  outside every column
- **THEN** the group floats in one overlay, the flyout is no longer visible, and
  no empty popup is left over the workspace

#### Scenario: A tab dragged out keeps the flyout on the remaining panels [cfd_tab_tearoff]

- **WHEN** a tab is dragged out of an open compact-strip flyout whose group still
  has other visible panels and dropped outside every column
- **THEN** only that panel floats, the flyout stays visible, and the flyout's
  group keeps the remaining panels

#### Scenario: A tab that empties the flyout closes it [cfd_empty_tearoff]

- **WHEN** a tab is dragged out of an open compact-strip flyout whose group has
  no other visible panel and dropped outside every column
- **THEN** the panel floats and the flyout is no longer visible
