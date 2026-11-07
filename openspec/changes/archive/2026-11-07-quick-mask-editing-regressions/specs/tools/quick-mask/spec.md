## ADDED Requirements

### Requirement: Quick Mask separates coverage from its editing selection

Quick Mask SHALL own mask coverage separately from the temporary pixel selection used to constrain editing. Entry SHALL use the original selection to initialize coverage and clear its cached marching ants. New selections SHALL show ordinary marching ants and constrain supported mask painting, fills and filters without replacing existing coverage. Deselect SHALL clear only that constraint. Exit with Q SHALL discard the temporary constraint and select the edited mask coverage. A document geometry operation applying its existing selection-reset policy SHALL clear both states so exit cannot install coverage with stale geometry.

#### Scenario: Cached entry outline disappears [qm_cached_ants]
- **WHEN** a selection outline has already been rendered and Quick Mask is entered
- **THEN** the old marching ants disappear immediately and its coverage remains in the mask

#### Scenario: Temporary selections constrain edits without replacing the mask [qm_temporary_clip]
- **WHEN** a new selection is made in Quick Mask and used for a fill, brush stroke or filter, then deselected or exited with Q
- **THEN** edits affect only the selected area, untouched mask coverage survives, deselection removes only the editing constraint, and Q exposes mask coverage rather than the constraint

#### Scenario: Geometry resets do not retain stale mask coverage [qm_geometry_reset]
- **WHEN** a document resize, canvas resize, rotation or crop clears the selection during Quick Mask and the mode is exited
- **THEN** no old-sized mask is installed as the selection, and undo restores the previous document and mask selection correctly

### Requirement: Quick Mask undo restores visible independent state

Undo and redo through keyboard and history commands SHALL restore mask coverage and temporary editing selections independently. The canvas SHALL repaint the restored mask even when document pixels have not changed. Mode toggles SHALL not add history states, and leaving Quick Mask SHALL not expose a temporary constraint during later undo or redo.

#### Scenario: Keyboard undo repaints the mask [qm_keyboard_undo]
- **WHEN** real canvas strokes are committed in Quick Mask and undone or redone through keyboard shortcuts
- **THEN** the history position, displayed overlay, and mask bytes agree without leaving the mode

#### Scenario: History preserves the mask and editing constraint separately [qm_independent_history]
- **WHEN** temporary selections and mask edits are undone and redone inside Quick Mask and across exit
- **THEN** both states restore correctly inside the mode, and only mask coverage becomes the normal selection outside it

### Requirement: Quick Mask strokes retain coverage with bounded presentation

Each brush stroke SHALL preserve existing mask coverage outside its edited samples. Live stroke samples SHALL use the canvas damage/deferred-present path without full shell refreshes, full-frame invalidation, document recompositing, or per-pixel Qt color queries. Coverage sharing SHALL avoid materializing a fragmented mask for a small dab. CPU presentation SHALL blend the same premultiplied overlay values as before.

#### Scenario: Existing mask survives consecutive canvas strokes [qm_retain_strokes]
- **WHEN** Quick Mask is initialized from a selection and several real brush gestures edit different locations
- **THEN** earlier mask regions and untouched coverage survive every stroke, and layer pixels remain unchanged

#### Scenario: Brush samples present bounded damage [qm_bounded_strokes]
- **WHEN** a small brush gesture produces multiple samples on a larger Quick Mask
- **THEN** samples emit bounded region damage, the gesture is recognized as painting, shell/frame invalidation waits for commit, and fragmented coverage is not materialized

## MODIFIED Requirements

### Requirement: Entry initializes protection from the pixel selection

With a document open, the Quick Mask control and `Q` SHALL toggle Quick Mask mode. Entry SHALL preserve an existing pixel selection's exact coverage. Without an active pixel selection, entry SHALL show no protection anywhere under the default Masked Areas display. An explicitly empty selection SHALL remain distinct from no active selection. The default overlay SHALL be red at 50% opacity over fully protected areas, clear over fully selected areas, and proportional at partial coverage. Entry SHALL suppress marching ants for the selection used to initialize the mask. A temporary editing selection created inside Quick Mask SHALL display its own ordinary marching ants.

#### Scenario: No pixel selection starts clear [qm_entry_clear]
- **WHEN** Quick Mask is entered with a selected layer and no active pixel selection
- **THEN** the canvas has no red overlay, and black painting can create protected red areas anywhere on the canvas

#### Scenario: Existing selection protects its exterior [qm_entry_selection]
- **WHEN** Quick Mask is entered with a rectangular pixel selection
- **THEN** its interior is clear, its exterior has the red 50% overlay, and stored selection coverage is unchanged

#### Scenario: An explicit empty selection stays empty [qm_entry_empty]
- **WHEN** Quick Mask is entered with an explicit selection whose coverage is zero everywhere
- **THEN** the entire canvas is protected and tinted red at 50% opacity
