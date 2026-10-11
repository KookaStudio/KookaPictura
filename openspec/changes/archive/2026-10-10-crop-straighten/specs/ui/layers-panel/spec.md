# Spec Delta

## ADDED Requirements

### Requirement: Layers panel row cursors

The Layers panel SHALL show a pointing-hand cursor while the pointer hovers a
layer row, and SHALL restore the default cursor when the pointer leaves.

#### Scenario: Hovering a row shows the hand cursor [uip_layers_hover_cursor]

- **WHEN** the pointer moves over a layer row
- **THEN** the cursor is the pointing hand; moving off the panel restores the default cursor

### Requirement: Layers panel lock controls

A regular layer's lock badge SHALL be removable with a single click, and adding a
layer mask to a locked Background layer SHALL convert it to a regular layer and
remove its lock. The row lock badge SHALL be about one third smaller than the
thumbnail, and the header lock buttons SHALL match the filter-bar icon size. The
blend-mode select SHALL be no wider than needed for its values.

#### Scenario: Single click removes a layer lock [uip_layers_lock_click]

- **WHEN** a locked regular layer's lock badge is single-clicked
- **THEN** the layer becomes unlocked

#### Scenario: Masking a Background converts it [uip_layers_mask_unlocks]

- **WHEN** a mask is added to a Background layer
- **THEN** the layer becomes a regular layer with its lock removed

### Requirement: Layers panel thumbnail layout

Within a layer row the mask thumbnail SHALL sit immediately to the right of the
image thumbnail with a chain glyph between them (a broken-chain glyph when the
link is off), and a vector-mask thumbnail SHALL sit immediately to the right of
the mask thumbnail with the same chain glyph between them. The mask and vector
thumbnails SHALL have the same aspect ratio as the image thumbnail. A row's
active edit target SHALL be marked with brackets around that thumbnail, and
clicking a thumbnail SHALL make it the active target; Alt+clicking a thumbnail
SHALL also activate it in the composite. A group row SHALL NOT draw brackets
around its icon.

#### Scenario: Mask thumbnail adjacency [uip_layers_mask_adjacency]

- **WHEN** a layer with a mask and a vector mask is shown
- **THEN** the mask thumbnail is directly right of the image thumbnail and the vector mask directly right of the mask, each with a chain glyph

#### Scenario: Active thumbnail brackets [uip_layers_active_thumb]

- **WHEN** a layer thumbnail is clicked to become the edit target
- **THEN** brackets mark that thumbnail and a group row never shows brackets on its icon

### Requirement: Layers panel folder rows

A folder row SHALL be vertically shorter than a layer row, its folder icon SHALL
be about half the row thumbnail size, and its right column SHALL use less
left-side padding without changing child-layer visibility controls. A group row
SHALL show its expand/collapse chevron even when it has no children.

#### Scenario: Empty group is expandable [uip_layers_folder_chevron]

- **WHEN** a group with no children is shown
- **THEN** the row shows its chevron and can be expanded

### Requirement: Layers panel visibility toggle responsiveness

Toggling a layer's visibility SHALL take effect immediately and SHALL NOT block a
second toggle while the model refreshes.

#### Scenario: Two quick visibility toggles [uip_layers_visibility_responsive]

- **WHEN** a layer's eye is clicked twice in quick succession
- **THEN** both toggles apply
