## ADDED Requirements

### Requirement: Vector mask thumbnail context menu

Right-clicking a layer's vector-mask thumbnail SHALL open a vector-mask menu
with Disable Vector Mask (Enable Vector Mask while it is disabled), a
separator, Delete Vector Mask, and Rasterize Vector Mask. Each command SHALL act
on the clicked layer as one undo state.

#### Scenario: The vector mask menu [lmc_vector_menu]

- **WHEN** the vector-mask thumbnail of a layer with both masks is right-clicked, and its commands are chosen
- **THEN** the menu lists Disable Vector Mask, Delete Vector Mask and Rasterize Vector Mask, the first reads Enable Vector Mask while the mask is disabled, and Delete and Rasterize remove the vector mask, each as one state

### Requirement: The trash deletes the vector mask of a doubly-masked layer

When the single selected layer has both a layer mask and a vector mask, the
Layers panel's trash SHALL ask "Delete Vector Mask?" instead of deleting the
layer. OK SHALL delete only that layer's vector mask, keeping the layer and its
layer mask, and Cancel SHALL change nothing. The prompt SHALL offer "Don't show
again"; once that is checked and the prompt confirmed, later deletions SHALL go
ahead without asking. The selected layer SHALL decide this even when the
current row lies outside the selection. With fewer masks the trash SHALL delete
the selection as before.

#### Scenario: Deleting the vector mask from the trash [lmc_trash_vector_prompt]

- **WHEN** the trash is clicked on a layer with both masks: cancelled once, then confirmed with "Don't show again", then clicked again after a new vector mask
- **THEN** the cancel keeps both masks, the confirm removes only the vector mask, and the last click removes the new vector mask without a prompt; the layer stays throughout

#### Scenario: The trash reads the selected row [lmc_trash_selected_row]

- **WHEN** a doubly-masked layer is selected, the current row is moved to another layer with Ctrl+arrow, and the trash is clicked
- **THEN** the prompt appears, and only the selected layer's vector mask is deleted; no layer is deleted
