# ui/view-menu Specification

## Purpose
The application-wide state behind the View menu's non-printing aids: the Extras master, the Show submenu, the Show Extra Options membership window, and the grid, pixel-grid, layer-edge, target-path, selection-edge, color-sampler and zoom entries introduced with it.

## Requirements

### Requirement: View overlay state

The application SHALL keep one application-wide View overlay state containing an
enabled flag for each `View > Show` entry available in the running edition
(Selection Edges, Target Path, Layer Edges, Guides, Grid, Count in Extended,
Smart Guides, Slices, Notes, Pixel Grid), one color-sampler visibility flag, one
`Extras` master flag, and one membership set for the `Show Extra Options`
window. The defaults SHALL be enabled for every entry except Grid, samplers on,
`Extras` on, and every listed entry a member. The state SHALL persist in the
session store, SHALL apply to every open canvas and to documents opened later,
and MUST NOT change document pixels, history, or saved output. The Show entries,
`Extras`, `Show > All`, `Show > None` and the zoom presets SHALL be disabled when
no document is open; a disabled dispatch SHALL leave the stored state unchanged.
`Show Extra Options` and `Lock Slices` SHALL remain available without a document
because they are application-global. A handler for a document-scoped command
SHALL itself check that a document is open, because dispatch does not honour the
enabled state.

#### Scenario: Fresh defaults [vm_defaults]

- **WHEN** the application starts with no stored View state
- **THEN** every Show entry available in the running edition except Grid is enabled, Grid is disabled, color samplers are enabled, Extras is checked, and every listed entry is a member of the Extras set

#### Scenario: The state survives a restart [vm_persist]

- **WHEN** the user hides Guides and Slices, shows Grid, turns Extras off, unchecks Notes as a member, and in Extended hides Count, then restarts
- **THEN** each stored value, including the Slices, Notes and Count flags, is restored and the menu checkmarks reflect it

#### Scenario: Toggling applies to every canvas [vm_apply_all]

- **WHEN** two documents are open and the user toggles Layer Edges off
- **THEN** both canvases stop drawing layer edges and neither document records a history state

#### Scenario: Overlays apply to every canvas [vm_apply_all_overlays]

- **WHEN** two documents are open with user slices, notes and Count marks in Extended, and the user turns Show > Slices, Show > Notes and Show > Count off
- **THEN** both canvases hide the slice, note and count overlays and neither document records a history state

#### Scenario: Document-scoped commands are inert without a document [vm_no_document]

- **WHEN** no document is open and a Show command, Extras, All, None or a zoom preset is dispatched
- **THEN** the command is disabled, nothing changes on screen, and the stored state is unchanged

#### Scenario: Application-global commands need no document [vm_global_commands]

- **WHEN** no document is open and the user opens Show Extra Options and toggles Lock Slices
- **THEN** the window opens, the lock flag flips and persists, and the document-scoped Show and zoom commands stay disabled

#### Scenario: Later and loading documents receive the state [vm_open_later]

- **WHEN** the user changes the state, opens another document, and separately holds an open in flight with `setOpenDelayForTest` while toggling
- **THEN** the new document receives the state when its canvas is created and the in-flight open does not crash

#### Scenario: The window follows the edition [vm_edition]

- **WHEN** the application runs in Standard and then Extended
- **THEN** Show Extra Options lists nine entries in Standard and ten in Extended, with Count available only in Extended, while 3D Axis is excluded from the window in both editions because it has no engine and stays disabled

### Requirement: Extras master and Show All/None

`View > Extras` (`Ctrl+H`) SHALL be a checkable command, on by default. While it
is off, every enabled extra that is a member of the Show Extra Options set SHALL
be hidden; a non-member extra SHALL keep its own visibility. Turning Extras off
SHALL NOT change any Show checkmark, membership, or document state; turning it on
SHALL restore the enabled member extras. Color samplers SHALL always be members
and their visibility flag SHALL participate in the master. `View > Show > All`
SHALL enable every extra and the sampler flag, and turn Extras on. `View > Show >
None` SHALL disable every extra and clear the sampler flag. Membership SHALL NOT
restrict `All` or `None`. An owning tool MAY override only the Show per-item flag
for its own overlay; it SHALL NOT override Extras.

#### Scenario: Extras hides members only [vm_extras_members]

- **WHEN** Guides and Grid are enabled, Grid is not a member, and the user turns Extras off
- **THEN** Guides disappear, Grid stays visible, and both Show checkmarks remain checked

#### Scenario: Extras keeps per-item state [vm_extras_state]

- **WHEN** Extras is turned off and then on again
- **THEN** every Show checkmark is unchanged and the previously visible extras return

#### Scenario: All and None ignore membership [vm_all_none]

- **WHEN** Grid is not a member and Extras is off, and the user chooses Show > All
- **THEN** every extra is enabled, Extras is on, and Grid is visible; choosing Show > None then hides every extra and clears the sampler flag

### Requirement: Show Extra Options window

`View > Show > Show Extra Options` SHALL open a modal window with one checkbox
for each implemented Show entry available in the running edition, excluding
color samplers and 3D Axis, all checked by default, and text stating that the set
controls what `View > Extras` hides. The window SHALL list Selection Edges, Target
Path, Layer Edges, Guides, Grid, Smart Guides, Slices, Notes and Pixel Grid in
both editions, and Count in Extended only. 3D Axis SHALL NOT appear because it
has no engine and stays disabled. OK SHALL persist the membership set; Cancel
SHALL leave it unchanged. The set SHALL bound only the Extras master; `All` and
`None` SHALL ignore it. Color samplers SHALL always be members and SHALL NOT
appear in the window.

#### Scenario: The window edits membership [vm_options_window]

- **WHEN** the user opens Show Extra Options, verifies the edition's list, unchecks Grid, accepts, and restarts
- **THEN** every available entry was checked, Grid is no longer a member, and the stored membership retains that choice

#### Scenario: Cancel keeps the membership [vm_options_cancel]

- **WHEN** the user opens Show Extra Options, changes several checkboxes, and cancels
- **THEN** the membership set is unchanged

#### Scenario: Membership binds only Extras [vm_options_scope]

- **WHEN** Grid is enabled but not a member and the user turns Extras off, then chooses Show > All
- **THEN** Grid stays visible while Extras is off and becomes visible again after Show > All, which also affects non-members

### Requirement: Selection Edges scope

`View > Show > Selection Edges` SHALL be a checkable command. The flag SHALL
persist in the View state and SHALL be on for each new selection. Toggling it
SHALL show or hide the marching ants of the current selection only and SHALL NOT
delete or alter the selection. A genuinely new selection SHALL show the edges
again; in-place mask edits, selection moves, Undo/Redo restores and Quick Mask
updates SHALL NOT re-show hidden edges. `View > Extras` off SHALL hide selection
edges regardless of the toggle, and turning Extras on SHALL restore them only
when the toggle is on. The setting SHALL apply to every canvas, and each document
SHALL track its own last-seen selection generation.

#### Scenario: A new selection restores the edges [vm_selection_edges]

- **WHEN** the user hides Selection Edges with a selection active and then makes a different selection
- **THEN** the first selection loses its ants without changing, and the new selection shows ants again

#### Scenario: In-place edits do not restore the edges [vm_selection_edges_no_reset]

- **WHEN** Selection Edges is hidden and the user edits the selection mask in place, moves the selection, Undoes or Redoes, or updates Quick Mask
- **THEN** the ants stay hidden, and only a genuinely new selection shows them again

#### Scenario: The generation is tracked per document [vm_selection_edges_documents]

- **WHEN** two documents have selections and the user hides Selection Edges, then makes a new selection in one document
- **THEN** both documents hide while the flag is off, and the new selection re-enables the ants for both documents' selections

#### Scenario: Extras overrides Selection Edges [vm_selection_edges_extras]

- **WHEN** Selection Edges is on and the user turns Extras off, then on
- **THEN** the ants disappear and return with the Show toggle still checked

### Requirement: Grid overlay and preferences

`View > Show > Grid` SHALL be a checkable command, off by default. When on, the
canvas SHALL draw vertical and horizontal grid lines inside the document rect
only, following pan, zoom and view rotation, using the grid colour, style,
interval and subdivisions from the Guides, Grid, & Slices preferences. The Grid
preferences group SHALL be live: Color (Light Blue, Light Red, Green, Medium
Blue, Yellow, Magenta, Cyan, Light Gray, Black, Custom…; default Light Gray),
Style (Lines, Dashed Lines, Dots; default Lines), Gridline Every (a number with
Pixels, Inches, Centimeters, Millimeters, Points, Picas or Percent; default 1
inch) and Subdivisions (an integer of at least 1; default 4). A change SHALL
apply to every open canvas and persist. Percent SHALL divide the document's
width and height evenly. Extras off SHALL hide the grid without changing the
Show toggle.

#### Scenario: The grid draws at the configured interval [vm_grid_toggle]

- **WHEN** a 72 ppi document shows Grid with 1 inch intervals and 4 subdivisions
- **THEN** major lines are 72 document pixels apart, minor lines every 18 pixels, and turning Grid off removes them

#### Scenario: Grid preferences apply and persist [vm_grid_prefs]

- **WHEN** the user picks Magenta, Dots, 25%, and 5 subdivisions and reloads the session
- **THEN** every open canvas draws that grid and the stored values match

#### Scenario: Percent spacing divides evenly [vm_grid_percent]

- **WHEN** the grid is set to 25% on a 400×200 document
- **THEN** major lines sit at 100 pixels horizontally and 50 pixels vertically

#### Scenario: Extras hides the grid [vm_grid_extras]

- **WHEN** Grid is on and Extras is turned off, then on
- **THEN** the grid disappears and returns with the Show checkmark unchanged

### Requirement: Pixel Grid overlay

`View > Show > Pixel Grid` SHALL be a checkable command, on by default. While it
is on and the view zoom is above 500%, the canvas SHALL draw a one-document-pixel
grid over the visible document, following pan, zoom and view rotation, and SHALL
hide it at 500% or below. Extras off SHALL hide it without changing the Show
toggle.

#### Scenario: The threshold is above 500% [vm_pixel_grid_threshold]

- **WHEN** the canvas is viewed at 500% and then at 600%
- **THEN** no pixel grid is drawn at 500% and one cell per document pixel is drawn at 600%

#### Scenario: The toggle and Extras hide the pixel grid [vm_pixel_grid_toggle]

- **WHEN** the canvas is at 800% and Pixel Grid is turned off, then on, then Extras is turned off
- **THEN** the grid disappears, returns, and disappears again with the Show checkmark unchanged

### Requirement: Layer Edges overlay

`View > Show > Layer Edges` SHALL be a checkable command, on by default. When on,
the canvas SHALL outline the active layer's content bounds with a thin
non-printing border that follows pan, zoom and view rotation. A target change, a
layer selection, or a document edit SHALL update the outline; while the Move tool
previews a layer move, the outline SHALL follow the preview offset, and a
selection-content move SHALL leave it in place. No active layer or an empty
content rect SHALL draw nothing. Extras off SHALL hide the outline without
changing the Show toggle.

#### Scenario: The active layer is outlined [vm_layer_edges]

- **WHEN** a small pixel layer is the active layer on a larger canvas and Layer Edges is on
- **THEN** the outline matches the layer's content rect and not the canvas rect

#### Scenario: Changing the target moves the outline [vm_layer_edges_target]

- **WHEN** the active layer switches from one pixel layer to another at a different position
- **THEN** the outline follows the new active layer's content bounds

#### Scenario: A document edit moves the outline [vm_layer_edges_edit]

- **WHEN** the active layer's pixels change so its content bounds grow
- **THEN** the outline updates to the new content rect

#### Scenario: Dragging a layer moves its outline [vm_layer_edges_drag]

- **WHEN** the Move tool drags a shape layer with Layer Edges on
- **THEN** the dashed outline follows the preview offset during the drag and lands at the committed content bounds

#### Scenario: Selecting another layer moves the outline [vm_layer_edges_active]

- **WHEN** the Layers panel selects a different shape layer with Layer Edges on
- **THEN** the dashed outline shows the newly selected layer's content bounds before any document edit

### Requirement: Target Path overlay

`View > Show > Target Path` SHALL be a checkable command, on by default. When on
and a non-path tool is active, the canvas SHALL draw the Paths panel's active
path (Work Path, saved path, or shape layer path) as a curve following pan, zoom
and view rotation. The path tools SHALL keep their editing overlay with anchors
and handles, and the base curve SHALL NOT draw while a path-editing tool is
active. A target change, a path edit, a layer selection, or a move or transform
of the layer whose outline is the target SHALL update the curve; while the Move
tool previews such a layer, the curve SHALL follow the preview offset. No active
path or an empty path SHALL draw nothing. Extras off SHALL hide the curve without
changing the Show toggle.

#### Scenario: The target path shows with another tool [vm_target_path]

- **WHEN** a Work Path exists and the Move tool is active with Target Path on
- **THEN** the path curve is drawn without the Pen tool's anchors and handles

#### Scenario: The toggle and Extras hide the target path [vm_target_path_off]

- **WHEN** Target Path is turned off and then on, and Extras is then turned off
- **THEN** the curve disappears, returns, and disappears again with the Show checkmark unchanged

#### Scenario: A path edit updates the curve [vm_target_path_edit]

- **WHEN** the active path is edited or the Paths panel selects another path
- **THEN** the persistent curve shows the new target and geometry

#### Scenario: A path tool owns its overlay [vm_target_path_path_tool]

- **WHEN** the Pen or Path Selection tool is active with Target Path on
- **THEN** only the tool's editing overlay is drawn, with no duplicate base curve

#### Scenario: Moving a shape layer moves its target path [vm_target_path_layer_move]

- **WHEN** a shape layer's outline is the target path and the Move tool moves the layer
- **THEN** the persistent curve follows the moved outline

#### Scenario: Dragging a shape layer moves its target path [vm_target_path_layer_drag]

- **WHEN** a shape layer's outline is the target path and the Move tool drags the layer
- **THEN** the persistent curve follows the preview offset during the drag and lands at the committed position

#### Scenario: Selecting another shape layer moves the target path [vm_target_path_active_layer]

- **WHEN** the Layers panel selects a different shape layer whose outline is the target path
- **THEN** the persistent curve shows the newly selected layer's outline before any document edit

### Requirement: Color sampler gating

Color samplers SHALL have one persistent visibility flag and SHALL always be
members of the Extras set even though they have no Show entry and no membership
checkbox. `View > Show > All` SHALL set the flag, `View > Show > None` SHALL clear
it, and `Extras` off SHALL hide them while Extras on restores the flag's state.
Their visibility SHALL NOT be controlled by any other Show entry.

#### Scenario: Samplers follow All and None [vm_color_samplers]

- **WHEN** two color samplers exist with Extras on and the user chooses None, then All
- **THEN** the samplers hide with None and show with All

#### Scenario: Extras gates the samplers [vm_color_samplers_extras]

- **WHEN** the samplers are visible and the user turns Extras off, then on
- **THEN** the samplers hide and return with the sampler flag unchanged

### Requirement: View zoom presets

`View > 200%` SHALL set the active canvas zoom to 200%. `View > Print Size` SHALL
set it so one image inch maps to one logical screen inch using `96 / ppi`, the
Zoom tool context menu's rule. Both commands SHALL be non-checkable, SHALL apply
to the active canvas only, and SHALL be disabled without a document. They SHALL
match the Zoom tool context menu's 200% and Print Size behavior.

#### Scenario: 200% sets the zoom [vm_zoom_200]

- **WHEN** the active canvas is at 50% and View > 200% is dispatched
- **THEN** the canvas zoom becomes 2.0

#### Scenario: Print Size uses the document resolution [vm_print_size]

- **WHEN** View > Print Size is dispatched on 72 ppi and 300 ppi documents
- **THEN** the zoom becomes 96 / 72 and 96 / 300 respectively

#### Scenario: Zoom presets need a document [vm_zoom_no_document]

- **WHEN** no document is open
- **THEN** both commands are disabled and dispatch leaves the application unchanged
