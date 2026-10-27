# layers-panel Specification

## Purpose
Layer rows, property editing, layer operations, thumbnails, grouping, and the Layers panel dock.

## Requirements

### Requirement: Layers panel rows

The system SHALL present the full layer tree of the active document as rows in
topmost-first display order, each row showing a visibility toggle, a thumbnail
(or a folder glyph for a group), an editable name, and indicators for clipping,
a layer mask, and an adjustment/style badge. The visibility toggle SHALL be the
row's only visibility control: the row SHALL NOT paint a separate checkbox or
check indicator. A group row SHALL be expandable and collapsible, and its
children SHALL be shown indented beneath it when expanded; a layer clipped to
the layer below SHALL be indented and its base layer's name SHALL be underlined.
The rows SHALL reflect the document and update when the document changes or the
active document changes.

#### Scenario: Rows reflect the tree [m39_tree]

- **WHEN** a document with a group containing children is active
- **THEN** the panel shows a row per node, topmost-first, with the group's
  children shown indented under it and the group row marked expandable

#### Scenario: A clipped layer is indented under its base [m39_badges]

- **WHEN** a layer is clipped to the layer below it
- **THEN** the clipped row is indented and the base layer's name is underlined

#### Scenario: No document [m39_tree]

- **WHEN** no document is open
- **THEN** the panel shows no rows and does not crash

#### Scenario: A row paints no checkbox indicator [lpr_eye]

- **WHEN** a layer row is shown
- **THEN** its only visibility control is the eye icon and the model reports no
  check state for the row

### Requirement: Layer property editing

The system SHALL apply visibility, name, blend mode, opacity, fill, lock, and
color-label edits to the active document through the bridge, addressed by the
layer's path (a `/`-separated sequence of bottom-first child indices). Every
edit SHALL mark the document modified and be undoable. A blend-mode edit SHALL
record one undo state whose label is `Blending Change: <mode name>`, using the
mode's CS6 display name (for example `Blending Change: Color Burn`). When more
than one layer is selected, a header edit SHALL apply to every selected layer in
a single undo step, and a layer that the CS6 contract excludes SHALL be skipped
rather than fail the edit. The system SHALL report and edit opacity and fill
through the bridge as a value in `0..=255`, where `255` is fully opaque; the
panel SHALL present both as percentages per the Opacity and Fill percent
controls requirement. A layer's lock state SHALL be reported and edited per flag
(transparency, image pixels, position, nesting, or all). A layer's color label
SHALL be one of `None`, `Red`, `Orange`, `Yellow`, `Green`, `Blue`, `Violet`, or
`Gray`. The system SHALL refuse, rather than apply, an edit that the CS6
contract excludes: a group has no Fill control, and the Background layer SHALL
expose neither opacity nor fill and SHALL refuse lock and color-label edits. A
fully locked layer SHALL refuse opacity and fill edits; its lock flags SHALL
remain editable so it can be unlocked. A refused edit SHALL leave that layer
unchanged.

#### Scenario: Change opacity

- **WHEN** the user changes a layer's opacity in the panel
- **THEN** the document's layer opacity changes, the composite updates, and the document reports modified

#### Scenario: Change blend mode

- **WHEN** the user selects a blend mode for a layer
- **THEN** the layer's blend mode changes and the composite updates

#### Scenario: A blend change records its mode name [lpr_blend_history]

- **WHEN** a layer's blend mode is set to Color Burn
- **THEN** one undo state is recorded labelled `Blending Change: Color Burn`

#### Scenario: Rename a layer [m39_rename]

- **WHEN** the user edits a layer's name
- **THEN** the layer name changes and the tab title reflects an untitled-name change where applicable

#### Scenario: Change fill opacity

- **WHEN** the user changes a layer's fill opacity to 128 in the panel
- **THEN** the layer's fill becomes 128, the composite updates with the layer's pixels at half strength, and the edit is undoable as one step

#### Scenario: Toggle a lock flag

- **WHEN** the user enables Lock Position for a layer and then enables Lock All
- **THEN** the layer reports the position bit and then all four lock bits, the panel disables opacity and fill, and each toggle is one undo state

#### Scenario: Set a color label

- **WHEN** the user picks Red from the row's color-label menu
- **THEN** the layer's color label becomes Red and the row reflects it

#### Scenario: Fill is unavailable for a group

- **WHEN** a group is selected
- **THEN** the Fill control is disabled and a fill edit is refused, while Opacity remains editable

#### Scenario: Background and fully locked layers refuse edits

- **WHEN** the Background layer or a fully locked layer is selected and opacity or fill is changed
- **THEN** the edit is refused, the document is unchanged, and the control is disabled

#### Scenario: A multi-selection edit applies to the eligible layers in one step [m39_multi]

- **WHEN** several layers are selected and a blend mode is chosen
- **THEN** every selected layer that is not the Background is changed, an
  ineligible layer is left unchanged, and the whole edit is one undo step

#### Scenario: A nested layer is edited by path [m39_tree]

- **WHEN** a layer inside a group is renamed
- **THEN** only that nested layer changes, addressed by its path, and the
  operation is one undo step

### Requirement: Layer operations

The system SHALL offer New Layer, New Group, Duplicate Layer(s), Delete
Layer(s), Group Layers, Ungroup Layers, and Move Layer Up/Down from the panel
and its menus, applying them to the active document through the bridge, and the
document's rows SHALL update afterwards. A new layer SHALL be an empty, fully
transparent raster layer at the document size and SHALL NOT change the
composite. A new group SHALL be an empty group with a Normal blend. New
Layer/New Group SHALL insert directly above the selected layer, or, when the
selected row is a group, as that group's topmost child; with no selection they
SHALL insert at the top of the stack, and the new row SHALL become selected.
The same insertion rule SHALL apply to an adjustment or fill layer created from
the Layers panel, the Adjustments panel, or a `Layer` menu, and the created
node SHALL become selected. Duplicating SHALL deep-copy each selected layer —
children, mask, adjustment data, and all attributes included — directly above
itself, naming the copy `"<name> copy"`, except that the copy of the Background
SHALL be an ordinary layer: not the Background, unlocked, and with an opaque
alpha channel. Deleting SHALL remove every selected, eligible layer. Move
Layer Up/Down SHALL swap the selected layer with its neighbour within its own
container and MUST refuse a move that would place the layer below the
Background. Each applied operation SHALL mark the document modified and be one
undoable step.

#### Scenario: Add a layer

- **WHEN** the user clicks New Layer with a layer selected
- **THEN** an empty transparent layer is inserted directly above it, becomes selected, the row count grows by one, and the composite is unchanged

#### Scenario: Add a layer inside a selected group [m39_tree]

- **WHEN** the user clicks New Layer with a group selected
- **THEN** an empty transparent layer is inserted as that group's topmost child
  and becomes the selected row

#### Scenario: Add a group

- **WHEN** the user clicks New Group with no layer selected
- **THEN** an empty group is inserted at the top of the stack, becomes selected, and a new row appears

#### Scenario: Duplicate a layer

- **WHEN** the user duplicates the selected layer
- **THEN** a deep copy named `"<name> copy"` is inserted directly above it, the row count grows by one, and the copy becomes selected

#### Scenario: Delete a layer

- **WHEN** the user deletes the selected layer
- **THEN** the layer is removed and the panel updates

#### Scenario: Move a layer

- **WHEN** the user moves the selected layer up or down from the panel or row menu
- **THEN** its order within its container changes, the rows reorder, and the composite updates

#### Scenario: Move Down at the Background floor is refused

- **WHEN** the layer directly above the Background is moved down
- **THEN** the order is unchanged, no history state is added, and the Background
  stays bottom-most

#### Scenario: Duplicating the Background makes an ordinary layer

- **WHEN** the Background is duplicated
- **THEN** `Background copy` sits above it as an unlocked pixel layer with no lock badge, and the original stays the Background

#### Scenario: An adjustment inserts above the active layer [lpo_anchor_adjustment]

- **WHEN** a layer in the middle of a group is selected and an adjustment layer is created
- **THEN** the adjustment is inserted immediately above that layer inside the same group, becomes selected, and is one undo state

#### Scenario: A fill inserts above the active layer [lpo_anchor_fill]

- **WHEN** a layer in the middle of the stack is selected and a Solid Color or Gradient fill layer is created
- **THEN** the fill is inserted immediately above that layer in the same container, becomes selected, and is one undo state

### Requirement: Layers panel docking and toggle
The system SHALL host the Layers panel in a registered dock with a stable
`objectName` and SHALL expose a `Window > Panels > Layers` toggle.

#### Scenario: Toggle the Layers panel
- **WHEN** the user toggles Layers from the Window menu
- **THEN** the panel is shown or hidden without changing the document

### Requirement: Thumbnails downsample without a full-size intermediate

Layer thumbnails SHALL be produced by downsampling the layer's channel data
directly, without first materializing a full-resolution RGBA image, so that
thumbnail cost is not proportional to the layer's full pixel buffer. The
thumbnail's visual result SHALL be unchanged.

#### Scenario: A thumbnail of a large layer avoids a full-resolution image

- **WHEN** a 24 px thumbnail is generated for a 4000×4000 layer
- **THEN** no full-resolution RGBA image is allocated, the thumbnail is visually
  unchanged, and generation completes within a small time budget

### Requirement: Layer grouping commands

The system SHALL offer Group Layers and Ungroup Layers from the panel, the row
context menu, the `Layer` menu, and the `Ctrl+G`/`Shift+Ctrl+G` shortcuts, acting
on the active document's currently selected layers. Group Layers SHALL wrap the
selection in one new group placed at the topmost selected layer's position,
preserving the selected layers' order, and SHALL name the group `"Group N"` where
N is one more than the highest existing `Group <number>` name. Group Layers SHALL
be refused, leaving the document unchanged, when the selection is empty, contains
the Background layer, contains a fully locked layer, or spans more than one
container. Ungroup Layers SHALL splice each selected group's children into the
parent at the group's position, preserving their order, and SHALL skip a selected
layer that is not a group. Each applied operation SHALL be one undoable step. The
keyboard shortcut path SHALL use the same selection-aware operation as the menu
path; it SHALL NOT operate on only the current row.

#### Scenario: Group a layer

- **WHEN** the user runs Group Layers on a selected layer
- **THEN** the layer is wrapped in a new group that occupies the layer's position and the wrapped layer becomes its only child

#### Scenario: Group a multi-selection [m39_multi]

- **WHEN** the user runs Group Layers on several selected layers that share a
  container
- **THEN** one group is created at the topmost selected position containing all
  of them in their existing order, in one undo step

#### Scenario: Ctrl+G groups the selection [lpr_ctrl_g]

- **WHEN** several layers are selected and `Ctrl+G` is pressed
- **THEN** the whole selection is wrapped in one group in one undo step, exactly
  as the menu Group Layers command does

#### Scenario: Group refuses the Background

- **WHEN** the selection contains the Background layer
- **THEN** Group Layers is refused and the document is unchanged

#### Scenario: Ungroup a group with children [m39_multi]

- **WHEN** the user runs Ungroup Layers on one or more selected groups with children
- **THEN** each group is replaced in place by its children in their existing order, in one undo step

#### Scenario: Ungroup skips a non-group

- **WHEN** the user runs Ungroup Layers on a selection that includes a pixel or adjustment layer
- **THEN** the non-group is left unchanged and remains selected

### Requirement: Layers panel action strip icons

The system SHALL give each Layers-panel action-strip button an icon from the
frozen layers asset set: `layers.link` (Link Layers), `layers.fx` (Layer Style),
`layers.mask` (Add Layer Mask), `layers.fillAdjustment` (New Fill / Adjustment
Layer), `layers.group` (New Group), `layers.newLayer` (New Layer), and
`layers.delete` (Delete). The buttons SHALL keep their text labels, and adding
an icon SHALL NOT change what each button does. The **Add Layer Mask** button
SHALL be active: a click SHALL add a `reveal-selection` mask when a selection
exists and a `reveal-all` mask otherwise, and an `Alt`-click SHALL add a
`hide-all` mask, each through the layer-mask bridge and each one undoable step.
The **Layer Style** (`fx`) button SHALL be active: a click SHALL open the Layer
Style dialog on Blending Options for the active layer when that layer can carry
a style, and an `Alt`-click SHALL toggle all effects through the layer-style
bridge in one undoable step. The **Link Layers** button SHALL be shown disabled
until its operation lands. When no document is open, every action-strip button
SHALL be disabled.

#### Scenario: The implemented strip buttons carry icons

- **WHEN** the Layers panel is shown
- **THEN** its mask, fx, fill/adjustment, group, new-layer, and delete buttons each carry their documented icon

#### Scenario: A deferred button is disabled

- **WHEN** the Layers panel is shown before the link operation exists
- **THEN** the link button is disabled

#### Scenario: No document disables the strip [lpc_strip_no_document]

- **WHEN** no document is open
- **THEN** the Link, fx, Mask, New Fill/Adjustment, New Group, New Layer, and
  Delete buttons are disabled

#### Scenario: The mask button adds a mask [lmk_strip]

- **WHEN** the user clicks the Add Layer Mask strip button with no selection
- **THEN** a reveal-all mask is added to the active layer in one undoable step

#### Scenario: The mask button respects a selection [lmk_strip_sel]

- **WHEN** the user clicks the Add Layer Mask strip button while a selection exists
- **THEN** a reveal-selection mask is added to the active layer in one undoable step

#### Scenario: Alt-clicking the mask button hides all [lmk_strip_alt]

- **WHEN** the user `Alt`-clicks the Add Layer Mask strip button
- **THEN** a hide-all mask is added to the active layer in one undoable step

#### Scenario: The fx button opens Blending Options

- **WHEN** the user clicks the Layer Style strip button with an active layer that can carry a style
- **THEN** the Layer Style dialog opens on Blending Options for that layer

#### Scenario: Alt-clicking the fx button toggles all effects

- **WHEN** the user `Alt`-clicks the Layer Style strip button
- **THEN** every layer's effects are shown when any were hidden and hidden when any were shown, as one undoable step

### Requirement: Layer tree projection and paths

The system SHALL address every layer by a path: a `/`-separated sequence of
non-negative decimal indices, each an index into the bottom-first `children`
vector of the node named by the preceding segments, starting from the
document's layer list. The empty string SHALL be invalid, a segment SHALL have
no leading zeros, and a path SHALL be positional rather than a stable
identifier. The bridge SHALL project the whole tree as a depth-first,
topmost-first row list with, for each row, its path, depth, name, kind,
visibility, blend mode, opacity, fill, lock flags, color label, clipping flag,
mask presence, adjustment presence, expandability, and child count. A
path-based operation on an out-of-range or malformed path SHALL fail or no-op
without panicking and without changing the document.

#### Scenario: A nested node resolves by path [unit: path_resolve_nested; m39_tree]

- **WHEN** a path `"0/2/1"` is resolved
- **THEN** it names `doc.layers[0].children[2].children[1]` and its depth is 2

#### Scenario: The projection is depth-first and topmost-first [m39_tree]

- **WHEN** the row list of a document whose bottom layer is a group with two children is read
- **THEN** the rows are the group followed by both children, the group's row
  precedes its children, and the topmost node is the first row

#### Scenario: An out-of-range path is refused [unit: path_resolve_rejects]

- **WHEN** a setter is called with a path whose final segment is past the end of its container
- **THEN** the call fails, no history state is added, and the document is unchanged

### Requirement: Multi-selection edits and refusals

The system SHALL support contiguous and non-contiguous selection in the tree and
SHALL apply a header edit to all selected rows as one undo step. The eligibility
of each selected layer SHALL follow the CS6 contract: the Background and a fully
locked layer are skipped for opacity and fill; a group is skipped for fill; the
Background is skipped for lock, blend, and color; the Background and a fully
locked layer are skipped for delete. Group Layers SHALL refuse the whole
operation when the selection contains the Background, contains a fully locked
layer, or spans more than one container. A batch that changes no layer SHALL
record no history and emit no change.

#### Scenario: A multi-selection blend is one undo step [m39_multi]

- **WHEN** a visible layer and the Background are selected and Multiply is chosen
- **THEN** the visible layer becomes Multiply, the Background is unchanged, and
  one undo restores the prior blend mode

#### Scenario: Group fill is skipped [m39_multi]

- **WHEN** a group and a pixel layer are selected and the fill value is changed
- **THEN** the pixel layer's fill changes, the group's fill is unchanged, and the
  edit is one undo step

#### Scenario: Delete skips a fully locked layer [m39_multi]

- **WHEN** a fully locked layer and an unlocked layer are selected and Delete is invoked
- **THEN** the unlocked layer is removed, the fully locked layer remains, and the
  deletion is one undo step

### Requirement: Solo visibility

The system SHALL implement `Alt`-click solo visibility on a row's visibility
toggle: the first `Alt`-click SHALL snapshot the visibility of every row and
then show only the clicked layer or group (with its ancestors, and for a group
its descendants that were visible in the snapshot), and a second `Alt`-click
SHALL restore the snapshot exactly, including a prior partial-hidden state. The
snapshot SHALL belong to the document it was taken in and SHALL NOT be
serialized with the document. Each direction SHALL be exactly one undoable
step, and the snapshot SHALL be discarded when a structural operation runs or
the document is replaced or closed. Switching to another document and back
SHALL leave the solo in place, with its snapshot still restorable.

#### Scenario: Alt-click solos a layer [m39_solo]

- **WHEN** the user `Alt`-clicks a layer's eye in a document with several visible layers
- **THEN** only that layer (and its ancestor groups) is visible, one undo state
  is added, and the prior visibility is remembered

#### Scenario: A second Alt-click restores exactly [m39_solo]

- **WHEN** the user `Alt`-clicks again while solo is active
- **THEN** every row's prior visibility is restored exactly, including layers
  that were hidden before the first `Alt`-click

#### Scenario: Solo survives undo via its history state [m39_solo]

- **WHEN** the user undoes a solo operation
- **THEN** the prior visibility of every layer is restored

#### Scenario: A solo belongs to its document [m39_solo_document]

- **WHEN** the user solos a layer, switches to another document, and switches
  back
- **THEN** the first document is still soloed and a second `Alt`-click restores
  its prior visibility

### Requirement: Inline rename with Tab navigation

The system SHALL begin inline name editing when the user double-clicks a row's
**name/label content band**, and SHALL NOT begin editing on the visibility,
chevron, thumbnail, lock, fx, or mask regions. The content band SHALL be the
`nameRect` geometry the delegate paints, corrected so it mirrors paint exactly
(including the +4 gap after the thumbnail and the right-edge badge/mask caps)
and is floored to a non-zero minimum width, so a click on an empty label area
still resolves to the name and not to a zero-width rect. A double-click that is
not on the eye/chevron/thumbnail/lock/fx/mask controls and not on a masked or
clipped indicator SHALL be treated as a rename for a normal layer. When the
double-clicked row is the Background layer, the system SHALL convert it to a
normal layer (through the name-and-color dialog) instead of renaming; for every
other layer it SHALL invoke the layer-style affordance, which is a documented
no-op while no Layer Style dialog exists. While editing, the system SHALL commit
the edit and move to the next visible row when the user presses `Tab`, and to
the previous visible row when the user presses `Shift+Tab`. At the last or first
visible row, the commit SHALL occur without wrapping.

#### Scenario: Double-click starts editing [m39_rename]

- **WHEN** the user double-clicks a row's name/label content band
- **THEN** an editor opens on that row and the document is unchanged until the
  edit commits

#### Scenario: Double-click anywhere in the label band renames [lpr_rename_band]

- **WHEN** the user double-clicks a non-Background row anywhere in the content
  band outside the eye/chevron/thumbnail/lock/fx/mask controls, including a
  zero-width-name row when thumbnails are off
- **THEN** the inline editor opens for that row

#### Scenario: Double-click on a control does not rename [lpr_rename_name_only]

- **WHEN** the user double-clicks the eye, chevron, thumbnail, lock, fx, or mask
  control
- **THEN** no inline editor opens and the control's own action runs

#### Scenario: Double-clicking the Background converts it [lpr_background_dblclick]

- **WHEN** the user double-clicks the Background row in the content band
- **THEN** the Background is converted through the name-and-color dialog rather
  than opening an inline editor

#### Scenario: Tab commits and moves down [m39_rename]

- **WHEN** the user edits a name and presses `Tab`
- **THEN** the name changes, the edit is one undo state, and the editor moves to
  the next visible row

#### Scenario: Shift+Tab moves up without wrapping [m39_rename]

- **WHEN** the user is editing the first visible row and presses `Shift+Tab`
- **THEN** the edit commits and editing ends without wrapping to the last row

### Requirement: Panel Options

The system SHALL provide a `Panel Options…` dialog from the panel menu with a
thumbnail size (`None`, `Small`, `Medium`, `Large`), thumbnail contents
(`Entire Document`, `Layer Bounds`), an `Expand New Effects` toggle, an
`Add "copy" to Copied Layers and Groups` toggle, and a `Use Default Masks on
Fill Layers` toggle. The defaults SHALL be `Medium`, `Entire Document`, and
enabled for the three toggles respectively, and all five values SHALL persist in
the session store, with an older or missing value loading the default. Thumbnail
contents SHALL be resolved when the thumbnail is built: `Layer Bounds` fills the
thumbnail from the layer's own bounds, while `Entire Document` places the
layer's content at its document position scaled into a document-sized thumbnail.
`Expand New Effects` SHALL have no visible effect until effect rows exist. When
`Add "copy"` is enabled, a duplicated layer or group SHALL be named
`"<name> copy"`; when disabled, the copy SHALL keep the original's name. When
`Use Default Masks on Fill Layers` is enabled, a fill layer created from the
panel or a `Layer` menu SHALL receive the active selection as a layer mask when
one exists, and otherwise a white document-sized layer mask; when disabled, the
new fill layer SHALL have no mask. The `Add Mask by Default` preference itself
belongs to the Adjustments panel capability and SHALL NOT be controlled by
`Panel Options`.

#### Scenario: Defaults on a fresh install [m39_options]

- **WHEN** the panel options are read with no saved session values
- **THEN** the thumbnail size is Medium, the thumbnail contents is Entire Document, Expand New Effects is enabled, Add "copy" is enabled, and Use Default Masks is enabled

#### Scenario: Thumbnail contents changes the thumbnail [m39_options]

- **WHEN** a small layer in a large document is shown with Layer Bounds and then with Entire Document
- **THEN** the Layer Bounds thumbnail is filled by the layer while the Entire Document thumbnail shows the layer smaller in its document context

#### Scenario: Options persist [m39_options]

- **WHEN** the user changes any panel-option value and the session is saved and reloaded
- **THEN** the panel restores those values, including Add "copy" and Use Default Masks

#### Scenario: The copy toggle names a duplicate [lpo_copy_name]

- **WHEN** `Add "copy"` is enabled and a layer named `Base` is duplicated
- **THEN** the copy is named `Base copy`

#### Scenario: Disabling the copy toggle keeps the source name [lpo_copy_name_off]

- **WHEN** `Add "copy"` is disabled and a layer named `Base` is duplicated
- **THEN** the copy keeps the name `Base`

#### Scenario: A fill layer takes the selection as a default mask [lpo_default_mask]

- **WHEN** `Use Default Masks` is enabled, a selection is active, and a fill layer is created
- **THEN** the created fill layer carries a layer mask derived from the selection

#### Scenario: A fill layer without a selection gets a white mask [lpo_default_mask_white]

- **WHEN** `Use Default Masks` is enabled, no selection is active, and a fill layer is created
- **THEN** the created fill layer carries a white document-sized layer mask

#### Scenario: A fill layer created without the option has no mask [lpo_default_mask_off]

- **WHEN** `Use Default Masks` is disabled, a selection is active, and a fill layer is created
- **THEN** the created fill layer has no layer mask

### Requirement: Panel and row menus

The system SHALL provide a panel menu and a row context menu. The panel menu
SHALL offer `Panel Options…`, New Layer, New Group, Duplicate Layer(s), Delete
Layer(s), Group Layers, Ungroup Layers, Move Layer Up, and Move Layer Down;
commands the system does not yet implement SHALL appear disabled with a
`— not implemented yet` tooltip rather than being hidden.

The row context menu SHALL be assembled **per layer kind** and SHALL emit one
separator between adjacent non-empty command blocks, in this order:

1. Blending Options… (style-capable rows) and Edit Adjustment… (adjustment
   rows).
2. Duplicate Layer(s), Delete Layer(s), Group from Layers…, New Layer, New
   Group, Ungroup Layers, Move Layer Up, and Move Layer Down.
3. Quick Export as PNG, then Export As… (every non-Background kind).
4. Artboard/Frame placeholders, then Convert to Smart Object (pixel, Background, and
   type rows enabled; group and adjustment rows disabled).
5. Rasterize Type, Rasterize Shape, Rasterize Layer, and Rasterize Layer Style
   (applicable kinds; the style row is not yet implemented and is disabled).
6. Add/Delete/Enable/Disable Layer Mask, Add/Delete/Enable/Disable Vector Mask,
   and Create/Release Clipping Mask.
7. Link Layers (or Unlink Layers when the row is linked) and Select Linked
   Layers.
8. Copy Layer Style, Paste Layer Style, and Clear Layer Style.
9. Copy Shape Attributes and Paste Shape Attributes.
10. Release from Isolation, only while isolation is active, near the bottom of
    the menu.
11. Merge Down (or Merge Layers for a multi-selection), Merge Visible, Merge
    Clipping Mask, and Flatten Image.
12. The Color Label submenu.

Each kind (pixel, background, group, adjustment, type, shape, smart object)
SHALL be offered the commands that apply to it, and SHALL NOT be offered
commands that do not apply. Every applicable command that is not yet implemented
SHALL still appear, rendered disabled, carrying a `— not implemented yet`
tooltip, so that implementing a command enables an existing row rather than
adding one. The commands common to every kind except the Background SHALL
include Rename and a color-label submenu containing `None`, `Red`, `Orange`,
`Yellow`, `Green`, `Blue`, `Violet`, and `Gray`.

The **Background row SHALL use a brief dedicated menu**: `Layer from
Background…`, `Duplicate Layer…`, `Delete Layer` (disabled), `Quick Export as
PNG`, `Export As…`, and `Convert to Smart Object`. It SHALL NOT offer Rename,
Blending Options…, the color-label submenu, merge/flatten, or any other row.

A shape row SHALL, in addition to its kind's commands, offer **Copy Shape
Attributes**, **Paste Shape Attributes**, and **Rasterize Shape**, each wired to
the shape bridge and acting on that row. A shape row SHALL be resolved by its
shape-layer projection (a fill cut to a vector mask), not by its adjustment
kind, so it is not offered the adjustment-only `Edit Adjustment…` row.

A pixel row SHALL offer **Convert to Smart Object**, wired to the convert bridge
and acting on that row; it SHALL be enabled only when the row is a convertible
raster pixel layer, and a successful conversion SHALL select the new smart
object. A smart-object row SHALL, in addition to the common and pixel commands,
offer **Reset Transform**, **Convert to Layers**, **New Smart Object via Copy**,
**Edit Contents**, **Replace Contents…**, and **Export Contents…**, each wired
to its bridge and acting on that row, plus the not-yet-implemented
**Relink to File…** and **Convert to Linked…** rows rendered disabled with the
`— not implemented yet` tooltip. Reset Transform and Convert to Layers SHALL be
enabled only when the row's embedded source parses as a PSD/PSB document; New
Smart Object via Copy SHALL be enabled for a non-group, non-adjustment
`Embedded` smart object with a payload; Edit Contents SHALL be enabled only when
the row's embedded source parses as a PSD/PSB document; Replace Contents SHALL
be enabled only for a replaceable `Embedded` object with a payload; and Export
Contents SHALL be enabled only for a smart object with a non-empty payload. A
non-smart row SHALL NOT offer the smart-object-only rows. Each applied action
SHALL be one undoable step.

For a pixel, group, or adjustment/fill row the menu SHALL offer **Add Layer Mask**,
**Delete Layer Mask**, **Enable Layer Mask**, and **Disable Layer Mask**, each
wired to the layer-mask bridge: Add Layer Mask SHALL add a `reveal-selection`
mask when a selection exists and a `reveal-all` mask otherwise; Delete Layer
Mask SHALL remove the mask; and Enable/Disable Layer Mask SHALL set the mask's
enabled state. A pixel, group, or adjustment/fill row SHALL also offer **Add Vector Mask**, **Delete
Vector Mask**, **Enable Vector Mask**, and **Disable Vector Mask**, each wired
to the vector-mask bridge and gated by vector-mask presence. A pixel or
adjustment/fill row SHALL offer **Create Clipping Mask** when the row can clip,
and **Release Clipping Mask** when it is clipped. Each applied action SHALL be one undoable step.

A pixel, group, or adjustment/fill row SHALL offer **Merge Down** when a single row is targeted,
**Merge Layers** when several rows are targeted, **Merge Visible**, **Merge
Clipping Mask**, and **Flatten Image**, wired to their merge bridges; Merge
Down/Merge Layers SHALL be enabled only where the selection can merge. A pixel
row SHALL offer **Link Layers** when it is not linked and **Unlink Layers** when
it is, plus **Select Linked Layers**. **Group from Layers…** SHALL open the
Group from Layers dialog for the row's selection. A row with a layer style
SHALL offer the style rows. **Rasterize Type** SHALL rasterize a type row,
**Rasterize Shape** a shape row, and **Rasterize Layer** a smart-object or
fill-content row; a plain pixel row and a group SHALL hide the rasterize rows.
**Rasterize Layer Style** SHALL appear disabled with the `— not implemented
yet` tooltip on a styled row. Right-clicking an unselected row SHALL select it
before a row command runs, so every row action acts on the clicked row.

Right-clicking the visibility toggle SHALL offer show/hide this layer only and
show/hide all. The bottom action strip SHALL remain exactly the CS6 seven
buttons, so Move Up/Down SHALL NOT appear in the strip.

#### Scenario: The panel menu contains the wired commands [m39_menus]

- **WHEN** the panel menu is opened
- **THEN** it contains Panel Options, New Layer, New Group, Duplicate, Delete,
  Group, Ungroup, Move Up, and Move Down; commands that are not yet implemented
  are shown disabled with the `— not implemented yet` tooltip

#### Scenario: The row menu sets a color label [m39_menus]

- **WHEN** the user right-clicks a row and picks Red from the color-label submenu
- **THEN** that layer's color label becomes Red

#### Scenario: Move Up/Down reorders from the menu [m39_menus]

- **WHEN** the user picks Move Layer Up for a row that is not the top of its container
- **THEN** the layer swaps with the layer above it within its container in one undo step

#### Scenario: The action strip has no Move buttons [m39_strip]

- **WHEN** the Layers panel is shown
- **THEN** its bottom strip contains exactly the seven CS6 buttons and no Move Up
  or Move Down button

#### Scenario: Pixel and smart-object rows offer export

- **WHEN** the user opens the row menu for a pixel layer or a smart-object layer
- **THEN** it contains Export As… and Quick Export as PNG, with Quick Export first

#### Scenario: Group and adjustment rows offer export

- **WHEN** the user opens the row menu for a group or an adjustment layer
- **THEN** it offers Quick Export as PNG and Export As…

#### Scenario: Type rows offer export

- **WHEN** the user opens the row menu for a type layer
- **THEN** it offers Quick Export as PNG and Export As…

#### Scenario: A shape row offers its shape commands

- **WHEN** the user opens the row menu for a shape layer
- **THEN** it contains Copy Shape Attributes, Paste Shape Attributes, and
  Rasterize Shape, enabled and wired to the shape bridge

#### Scenario: A shape row is not offered the adjustment edit

- **WHEN** the user opens the row menu for a shape layer
- **THEN** it does not contain Edit Adjustment…

#### Scenario: A pixel row offers Convert to Smart Object [lpr_convert_row]

- **WHEN** the user opens the row menu for a convertible raster pixel layer
- **THEN** it contains Convert to Smart Object, enabled and wired to the convert
  bridge, and picking it converts the layer and selects the new smart object

#### Scenario: An existing smart object disables Convert to Smart Object

- **WHEN** the user opens the row menu for a layer that is already a smart object
- **THEN** it offers Convert to Smart Object, disabled, because the row is not a
  convertible raster pixel layer

#### Scenario: A smart-object row offers its smart-object commands [lpr_smart_rows]

- **WHEN** the user opens the row menu for a smart-object layer
- **THEN** it contains Reset Transform, Convert to Layers, New Smart Object via
  Copy, Edit Contents, Replace Contents…, and Export Contents…, enabled and wired
  to their bridges

#### Scenario: A smart-object row offers the linked rows disabled [lpr_smart_linked]

- **WHEN** the user opens the row menu for a smart-object layer
- **THEN** it contains Relink to File… and Convert to Linked…, each disabled and
  showing the `— not implemented yet` tooltip

#### Scenario: A plain pixel row omits the smart-object commands

- **WHEN** the user opens the row menu for a plain pixel layer
- **THEN** it does not contain Reset Transform, Convert to Layers, New Smart
  Object via Copy, Edit Contents, Replace Contents…, Export Contents…, Relink to
  File…, or Convert to Linked…

#### Scenario: A pixel row offers its full command set

- **WHEN** the user opens the row menu for a pixel layer
- **THEN** it contains the common commands, the color-label submenu, `Export
  As…`, `Quick Export as PNG`, and the pixel-applicable mask, style, clipping,
  smart-object, and rasterize commands — each either enabled or disabled with a
  tooltip when not yet implemented

#### Scenario: The row menu adds and deletes a mask [lmk_rowmenu]

- **WHEN** the user picks Add Layer Mask for a pixel row, then Delete Layer Mask
- **THEN** a mask is added and then removed from the active layer, each in one undo step

#### Scenario: The row menu toggles mask enablement [lmk_rowmenu_enable]

- **WHEN** the user picks Enable Layer Mask or Disable Layer Mask for a masked layer
- **THEN** the mask's disabled bit is cleared or set in one undo step

#### Scenario: A group row omits content commands

- **WHEN** the user opens the row menu for a group row
- **THEN** it does not offer `Export As…`, `Quick Export as PNG`, or fill
  commands, it offers `Ungroup Layers`, and it offers the layer-mask rows

#### Scenario: A type row offers rasterize

- **WHEN** the user opens the row menu for a type layer
- **THEN** it offers `Rasterize Type`

#### Scenario: An unimplemented applicable command is disabled, not absent

- **WHEN** the user opens the row menu for a kind and a command applies to that
  kind but is not yet implemented
- **THEN** the command is present, disabled, and shows the `— not implemented
  yet` tooltip

#### Scenario: The row blocks appear in CS6 order [lpr_row_blocks]

- **WHEN** the row menu is opened for a pixel layer with an editable style, a mask, and a selection that can merge
- **THEN** Blending Options, the layer operations, Quick Export/Export As, Convert to Smart Object, Rasterize, the mask and clipping rows, link rows, style rows, merge rows, and the color label appear in that order, with one separator between each non-empty block

#### Scenario: The Background row menu is brief [lpr_background_brief]

- **WHEN** the user opens the row menu for the Background layer
- **THEN** it contains only Layer from Background…, Duplicate Layer…, Delete Layer (disabled), Quick Export as PNG, Export As…, and Convert to Smart Object, and it does not contain Rename, Blending Options…, a color label, merge, or flatten

#### Scenario: A group row offers the mask and merge rows [lpr_group_rows]

- **WHEN** the user opens the row menu for a group
- **THEN** it offers Add/Delete/Enable/Disable Layer Mask, the four vector-mask rows, Quick Export/Export As, Link/Unlink Layers, Select Linked Layers, Merge Layers/Merge Visible/Merge Clipping Mask, and Flatten Image, and it does not offer fill rows

#### Scenario: A pixel row offers clipping and vector-mask rows [lpr_pixel_mask_rows]

- **WHEN** the user opens the row menu for a pixel layer that can clip
- **THEN** it offers Create Clipping Mask and the four vector-mask rows, and a clipped row offers Release Clipping Mask instead of Create Clipping Mask

#### Scenario: The row menu links and selects linked layers [lpr_link_rows]

- **WHEN** the user opens the row menu for an unlinked pixel row
- **THEN** it offers Link Layers and Select Linked Layers; once linked, the row offers Unlink Layers

#### Scenario: The row menu offers the merge family [lpr_merge_rows]

- **WHEN** the user opens the row menu for a single pixel row above another layer
- **THEN** it offers Merge Down, Merge Visible, Merge Clipping Mask, and Flatten Image, and a multi-selection offers Merge Layers instead of Merge Down

#### Scenario: The row menu rasterizes by kind [lpr_rasterize_rows]

- **WHEN** the user opens the row menu for a type layer, a shape layer, a fill layer, a smart object, and a plain pixel layer
- **THEN** the first four offer Rasterize Type, Rasterize Shape, Rasterize Layer, and Rasterize Layer respectively, the plain pixel row hides the rasterize rows, and a styled row also offers the disabled Rasterize Layer Style

#### Scenario: Release from Isolation sits near the bottom [lpr_isolation_row]

- **WHEN** isolation is active and the row menu is opened
- **THEN** Release from Isolation appears after the style and shape blocks and before the merge block

#### Scenario: A row action acts on the clicked row [lpr_row_action_target]

- **WHEN** the user right-clicks an unselected row and picks a row action
- **THEN** the panel selects that row first and the action applies to it

### Requirement: Layer row tooltips

The system SHALL provide a tooltip for each row that includes the layer's name
and its kind, where the kind is one of pixel, group, adjustment, background,
or type.

#### Scenario: A group row tooltip names the kind [m39_tooltip]

- **WHEN** the pointer rests on a group row
- **THEN** the tooltip contains the group's name and the word `group`

#### Scenario: A pixel row tooltip names the kind [m39_tooltip]

- **WHEN** the pointer rests on a pixel layer row
- **THEN** the tooltip contains the layer's name and the word `pixel`

### Requirement: Opacity and Fill percent controls

The Layers panel SHALL present Opacity and Fill as percentages in the range
`0..=100`, each through one reusable control that offers a text value, a popup
slider, and scrubbing by pressing and dragging horizontally on the label or the
field. Each field's `%` sign SHALL be rendered inside the value box rather than
beside it. The control SHALL convert between the displayed percentage and the
stored `0..=255` value with `round(pct * 255 / 100)` on edit and `round(value *
100 / 255)` on display, so `100`% is `255`, `50`% is `128`, and `0`% is `0`. The
control SHALL distinguish a live preview from a commit: while a scrub or slider
drag is in progress the control SHALL emit preview changes without adding a
history state, and when the edit finishes it SHALL emit one commit. The panel
SHALL coalesce applying those previews to at most one canvas update per short coalescing interval (about
120 ms), using the latest value, including while the pointer is held stationary
mid-drag. The interval SHALL start on the first change of a gesture and SHALL NOT
be restarted by later changes. A pending preview SHALL NOT be overwritten or discarded by a panel
sync: the field SHALL keep the user's latest value until the preview is applied
on its interval, and a panel refresh MUST NOT apply the pending preview early.
A gesture that returns to its starting value before the interval elapses SHALL
composite nothing and add no state when the final stored byte equals the
starting stored byte. A real view change, a document replacement in the same
view, or a change to the selected targets SHALL cancel the gesture and the
field's pending commit, whether the change happens before or after the preview
fires, and MUST NOT apply a value to the new target. A panel model reset SHALL
NOT by itself cancel the gesture: a transiently empty selection is not a target
change, but a restored non-empty selection whose layers differ SHALL cancel. The
commit SHALL be a no-op when the captured target no longer resolves. A canceled
gesture whose preview already changed pixels SHALL leave those pixels in the
document and mark it modified; it adds no history state. The commit SHALL stop
and discard any pending preview and apply the final value exactly once, adding
exactly one undo state for the whole edit. A completed text edit SHALL also add
one state. The control SHALL emit nothing for a programmatic sync, so a
programmatic sync does not feed back. The existing enable/refusal rules SHALL be
unchanged: Fill is unavailable for a group, and the Background and a fully
locked layer disable both controls. Synthetic display-only rows SHALL NOT count
as eligible targets.

#### Scenario: A percentage maps to the stored byte [lpc_percent]

- **WHEN** Opacity is set to 50 % on an eligible layer
- **THEN** the layer's stored opacity becomes 128 and the document is modified

#### Scenario: A stored byte displays as a percentage [lpc_percent]

- **WHEN** a layer with stored opacity 128 becomes the current row
- **THEN** the Opacity control displays 50 %

#### Scenario: The popup slider and label drag both change the value [lpc_percent]

- **WHEN** the user drags the Opacity popup slider or drags horizontally on the
  Opacity field
- **THEN** the percentage changes, previews are emitted, and the whole drag is
  applied in one undo step

#### Scenario: A drag adds one undo state only at release [lpc_preview]

- **WHEN** an Opacity or Fill drag previews several values and is then released
- **THEN** no undo state was added during the drag and exactly one undo state is
  added on release

#### Scenario: Rapid ticks coalesce into one preview [lpc_coalesced]

- **WHEN** an Opacity or Fill drag emits several values within one coalescing
  interval
- **THEN** at most one preview is applied for that interval, using the latest
  value, no undo state is added, and the interval is not restarted by later
  changes

#### Scenario: A mid-drag pause previews while the button is held [lpc_pause]

- **WHEN** the pointer stops moving during a drag but the button is still held
- **THEN** the latest value is previewed within the coalescing interval and no
  undo state is added

#### Scenario: A fast round trip with no stored-byte change does nothing [lpc_round_trip]

- **WHEN** a drag changes the value and returns to the starting value before the
  interval elapses, the final stored byte equals the starting stored byte, and
  the edit is released
- **THEN** no composite runs and no undo state is added

#### Scenario: The commit wins over a pending preview [lpc_commit_wins]

- **WHEN** the edit finishes while a preview is still pending
- **THEN** the commit applies the final value exactly once and adds exactly one
  undo state

#### Scenario: A panel sync preserves a pending preview [lpc_preserve]

- **WHEN** a panel refresh or model sync runs while a preview is pending
- **THEN** the field keeps the latest user value, no preview is applied early,
  and the pending preview is applied on its coalescing interval

#### Scenario: A target change cancels the gesture [lpc_target_change]

- **WHEN** the view changes, the document is replaced in the same view, or the
  selected targets change, before or after a preview fires
- **THEN** the gesture and the field's pending commit are canceled, no value is
  applied to the new target, and a preview that already ran leaves the document
  modified and dirty with no added history state

#### Scenario: The percent sign is inside the value box [lpr_percent]

- **WHEN** the Opacity or Fill field is shown
- **THEN** its `%` sign is drawn inside the value box rather than beside it

#### Scenario: A programmatic sync does not re-apply the edit [lpc_percent]

- **WHEN** the panel reflects an existing value into the control without user input
- **THEN** no edit is sent and no history state is added

### Requirement: Nesting lock

The system SHALL support a fifth lock flag, `nesting`, alongside transparency,
image pixels, position, and all. The flag SHALL be part of the layer's lock
state, reported in the row projection, and included in the "all" set, so that
Lock All also sets nesting and a fully locked layer is one whose four lower bits
are all set. A nesting-locked layer SHALL keep its structural parent: the system
SHALL refuse Group Layers and Ungroup Layers when the selection contains a
nesting-locked layer, leaving the document unchanged, while a within-container
reorder (Move Layer Up/Down) SHALL remain allowed. The flag SHALL round-trip
through the PSD `lspf` block.

#### Scenario: The nesting flag round-trips to the row and the bridge [lpc_nesting]

- **WHEN** the nesting lock is enabled for a layer and the layer's row is read
- **THEN** the row reports the nesting bit and `all` reports the four-bit set

#### Scenario: Grouping a nesting-locked layer is refused [lpc_nesting]

- **WHEN** Group Layers is run with a nesting-locked layer selected
- **THEN** the operation is refused, the layer's parent is unchanged, and no
  history state is added

#### Scenario: Ungrouping a nesting-locked group is refused [lpc_nesting]

- **WHEN** Ungroup Layers is run with a nesting-locked group selected
- **THEN** the operation is refused and the group and its children are unchanged

#### Scenario: Reordering within the container still works [lpc_nesting]

- **WHEN** Move Layer Up is run on a nesting-locked layer inside a group
- **THEN** the layer swaps with its sibling above it in one undo step

#### Scenario: The nesting bit survives a PSD round-trip [lpc_nesting]

- **WHEN** a layer with the nesting lock is written and read back
- **THEN** the nesting bit is set on the re-read layer

### Requirement: Layers panel header layout

The panel SHALL stack its header controls in this order, top to bottom: the
filter/search row; a row containing the blend-mode popup and a labeled Opacity
field; a row containing the five lock toggles and a labeled Fill field; the
layer list; and the action strip. The Opacity and Fill fields SHALL each carry a
leading text label, and each field's popup slider SHALL open centred
horizontally under the field rather than aligned to its right edge. The Opacity
field's label SHALL be right-aligned against its value box; the Fill field's
label SHALL keep its left alignment. The panel SHALL NOT provide its own menu
button; the wired Layer commands SHALL remain reachable from the panel-group
widget menu. The five lock toggles SHALL all sit in the single lock row above
the layer list, not below it.

#### Scenario: The header order is filter / blend+Opacity / locks+Fill / list [lpc_order]

- **WHEN** the Layers panel is shown
- **THEN** the filter row is above the blend and Opacity row, which is above the
  lock and Fill row, which is above the layer list, with the action strip below
  the list

#### Scenario: Opacity and Fill are labeled [lpc_labels]

- **WHEN** the Layers panel is shown
- **THEN** the Opacity field and the Fill field each have a text label directly
  to their left

#### Scenario: The Opacity label is right-aligned [lpc_opacity_label_right]

- **WHEN** the Layers panel header is laid out
- **THEN** the Opacity field's label text is right-aligned against its value box
  and the Fill field's label stays left-aligned

#### Scenario: The percent picker opens centered [lpc_popup]

- **WHEN** the user opens the Opacity or Fill slider popup
- **THEN** the popup is centered horizontally under its field

#### Scenario: The panel has no separate menu button [lpc_nomenu]

- **WHEN** the Layers panel header is inspected
- **THEN** it contains no panel-menu button, and the Layer commands are offered
  by the panel-group widget menu

### Requirement: Layers panel header fields

The Opacity and Fill fields SHALL each carry their label as part of the field,
and pressing and dragging that label horizontally SHALL scrub the value exactly
as dragging the field does. Each field's `%` sign SHALL be shown inside the
value box. The lock strip SHALL hold five toggles; the toggles for alpha, paint,
position, and nesting SHALL be semantic icons describing what they lock
(transparency, image pixels, position, nesting), and only the full-lock toggle
SHALL be a padlock icon.

#### Scenario: Dragging the label scrubs the value [lpr_label]

- **WHEN** the user presses the `Opacity` label and drags horizontally
- **THEN** the opacity percentage changes and is applied in one undo step, the
  same as dragging the field itself

#### Scenario: The value shows a percent sign inside the box [lpr_percent]

- **WHEN** the Opacity or Fill field is shown
- **THEN** its value's `%` sign is drawn inside the value box

#### Scenario: Only Lock All is a padlock [lpr_locks]

- **WHEN** the lock strip is shown
- **THEN** the alpha, paint, position, and nesting toggles use their semantic
  icons and the full-lock toggle is the only padlock

### Requirement: Layer drag and drop

A layer row SHALL be draggable. Starting a drag SHALL NOT extend the selection
or begin a rubber-band selection. The row model SHALL implement the drag-and-drop
capability virtuals — `mimeTypes()`, `canDropMimeData()`, and
`supportedDropActions()` — so the view's `canDrop()` is true and a real drop
indicator position is computed, and the view SHALL enter the dragging state on
drag enter so the indicator paints. **The model SHALL return
`Qt::ItemIsDropEnabled` for the invalid parent index**, so a top-level row is a
valid drop target surface and Qt's `AboveItem`/`BelowItem` indicator is
available for the top level as well as for nested rows; without it the drop
indicator is empty and reordering a top-level row shows nothing. **The view SHALL
set a closed-hand cursor for the duration of `startDrag`** and restore the
previous cursor when the drag ends. The system SHALL validate a candidate drop
during the drag and SHALL show a drop indicator only for a valid target, and it
SHALL reject an invalid drop without calling the bridge or changing the document.
Releasing a dragged row above or below another row SHALL reorder it at that
position, releasing it onto a group row SHALL reparent it as a child of that
group, and releasing it on the empty viewport below the last row SHALL move it to
the document root (out of any group), each as one undoable step. The system SHALL
refuse, leaving the document unchanged, a move of the Background layer, of a
fully-locked or nesting-locked layer, of a row onto itself, of a row into its
own descendant, or of a row to a position at or below a Background in the
destination container. Dropping a dragged row on a bottom-strip button SHALL apply that
button's action to the dragged row: Delete deletes it, New Layer duplicates it
unless the dragged row is the Background — in which case New Layer converts the
Background to a normal layer in place — and New Group groups it; buttons whose
operation is not yet implemented (mask, link, fx) SHALL be inert. A sibling
reorder SHALL resolve to an above/below drop and SHALL NOT be mistaken for a drop
into the row.

#### Scenario: Dragging reorders a row [lpr_drag]

- **WHEN** a row is dragged and released above a sibling
- **THEN** the row moves to that position in one undo step and the drag does not
  select any other row

#### Scenario: A top-level sibling shows the above/below indicator [lpr_drag_top_level]

- **WHEN** a top-level row is dragged over the gap above or below a top-level
  sibling
- **THEN** the model is drop-enabled for the invalid parent, the drop indicator
  is above/below, and the reorder is accepted

#### Scenario: The drag cursor is a closed hand [lpr_drag_cursor]

- **WHEN** a row drag starts
- **THEN** the viewport cursor is a closed hand until the drag ends, then the
  previous cursor is restored

#### Scenario: A sibling reorder resolves at the drop [lpr_drag_reorder_mode]

- **WHEN** a row is dragged over the gap above or below a sibling
- **THEN** the drop indicator is set to the above/below position and the reorder
  is accepted rather than being treated as a drop into the sibling

#### Scenario: The view enters the dragging state [lpr_drag_state]

- **WHEN** a drag enters the layer tree's viewport
- **THEN** the view sets its dragging state so the drop indicator is painted, and
  an invalid target shows no indicator

#### Scenario: Dragging onto a group reparents [lpr_drag]

- **WHEN** a row is dragged and released onto a group row
- **THEN** the row becomes a child of that group in one undo step

#### Scenario: Dragging out of a group to the viewport [lpr_drop_out]

- **WHEN** a row inside a group is dragged and released on the empty viewport
  below the last row
- **THEN** the row is reparented to the document root in one undo step

#### Scenario: An invalid target is rejected before commit [lpr_drop_rules]

- **WHEN** a row is dragged over its own descendant, onto itself, or over a
  refusal target such as the Background
- **THEN** no valid drop indicator is shown and releasing the drag changes
  nothing and adds no history state

#### Scenario: A drop below the Background is refused [lpr_drop_bg_below]

- **WHEN** a row is dragged over the gap below the Background in the same
  container
- **THEN** no drop indicator is shown, releasing the drag changes nothing, and
  no history state is added

#### Scenario: A locked or Background row is refused [lpr_drag]

- **WHEN** the Background, a fully-locked, or a nesting-locked row is dragged
  onto another row
- **THEN** the move is refused and the document is unchanged

#### Scenario: A press-move starts a drag rather than extending a selection [lpr_drag_enabled]

- **WHEN** the user presses a draggable row and moves the pointer
- **THEN** the drag pipeline starts, the selection is not extended, and no
  rubber-band selection appears

#### Scenario: Dropping on Delete deletes the dragged row [lpr_drop_button]

- **WHEN** a row is dragged onto the Delete strip button and released
- **THEN** that row is deleted in one undo step

#### Scenario: Dropping on New Layer duplicates the dragged row [lpr_drop_button]

- **WHEN** a non-Background row is dragged onto the New Layer strip button and
  released
- **THEN** the row is duplicated in one undo step

#### Scenario: Dropping the Background on New Layer converts it [lpr_drop_background_convert]

- **WHEN** the Background is dragged onto the New Layer strip button and released
- **THEN** the Background is converted to a normal layer in place in one undo
  step, rather than cloned as a locked `Background copy`

#### Scenario: Dropping on New Group groups the dragged row [lpr_drop_button]

- **WHEN** a row is dragged onto the New Group strip button and released
- **THEN** the row is wrapped in a new group in one undo step

### Requirement: Layer management commands

The system SHALL expose the layer-management commands from the Layers panel
menus and the `Layer` menu, wired to the active document through the bridge.
The set SHALL include Merge Down / Merge Layers (`Ctrl+E`), Merge Visible
(`Shift+Ctrl+E`), Merge Clipping Mask, Flatten Image, Layer from Background…,
Background From Layer, Layer via Copy (`Ctrl+J`), Layer via Cut
(`Shift+Ctrl+J`), Delete Hidden Layers, Hide Layers, Select Similar, Select
Linked Layers, Link Layers, Unlink Layers, and the Rasterize entries. Each
available command SHALL apply as exactly one undoable step and the panel rows
SHALL update afterwards. A command that does not apply to the current selection
SHALL be disabled rather than shown enabled and refused, and the `Rasterize`
Type, Shape, Vector Mask, Smart Object, Video, and 3D entries SHALL stay visible
and disabled. `Alt`-clicking the New Layer/New Group panel button SHALL open
the New Layer/New Group dialog instead of creating immediately.

#### Scenario: Merge is reachable from the Layer menu and the panel menu

- **WHEN** the `Layer` menu or the panel menu is opened with a document active
- **THEN** Merge Down / Merge Layers, Merge Visible, Merge Clipping Mask, and
  Flatten Image are present and wired to the bridge

#### Scenario: A selection-dependent command is enabled correctly

- **WHEN** one layer is selected and the Layer menu is opened
- **THEN** the merge command is enabled for Merge Down, and with multiple
  selected layers it is enabled for Merge Layers

#### Scenario: Alt-click opens the dialog

- **WHEN** the user Alt-clicks the New Layer panel button
- **THEN** the New Layer/New Group dialog opens instead of a layer being created
  immediately

### Requirement: Thumbnail Ctrl-click selects layer pixels

The layer row delegate SHALL expose the thumbnail's hit rectangle, and a
`Ctrl`-click inside that rectangle SHALL select that layer's pixels: the bridge
SHALL build a document-sized selection from the layer's alpha channel (`-1`,
treating a missing alpha channel as fully opaque `255`), offset by the layer's
`rect`, and combine it with the current selection using the New mode. The click
SHALL be consumed by the thumbnail and SHALL NOT start a drag, begin rename, or
toggle visibility.

#### Scenario: Ctrl-clicking a thumbnail selects its pixels [lpr_thumbnail_select]

- **WHEN** the user `Ctrl`-clicks a layer's thumbnail
- **THEN** a document-sized selection matching that layer's alpha shape becomes
  the current selection and the layer's row stays selected

#### Scenario: A thumbnail Ctrl-click is not a drag or rename [lpr_thumbnail_consumes]

- **WHEN** the user `Ctrl`-clicks a layer's thumbnail
- **THEN** no drag starts, no editor opens, and the visibility is unchanged

### Requirement: Nesting lock button is hidden in the panel

The Layers panel SHALL hide its nesting-lock toggle button (`lockNesting_`), so
the panel exposes only the transparency, pixel, position, and all lock controls.
Hiding the button SHALL NOT change the engine: the `NESTING` flag SHALL remain
part of the layer lock state, SHALL still refuse Group Layers and Ungroup Layers
on a nesting-locked layer, SHALL still round-trip through the PSD `lspf` block,
and SHALL still be settable and readable through the bridge. A layer already
carrying the flag SHALL still report it in its row and keep its structural
parent; the button is a presentation removal, not a feature removal.

#### Scenario: The panel has no nesting lock button [lpr_nesting_hidden]

- **WHEN** the Layers panel header lock controls are shown
- **THEN** the nesting-lock button is hidden and the other lock controls remain

#### Scenario: The engine nesting rules still hold [lpr_nesting_engine]

- **WHEN** a layer reports the `NESTING` bit programmatically and Group Layers
  is attempted
- **THEN** the operation is still refused and the PSD round-trip still preserves
  the bit

### Requirement: Layer row visibility gutter and content spacing

The visibility toggle SHALL be centred horizontally inside its fixed left
gutter, with equal padding on its left and right sides, rather than being
left-anchored with all padding on one side. The delegate SHALL draw a 1 px
separator in a slightly darker grey at the gutter's right edge, between the
visibility toggle and the row content. The expand/collapse chevron slot SHALL be
reserved only for rows that are expandable, so a regular (non-group) layer's
thumbnail begins immediately after the gutter instead of leaving an empty
chevron-width gap. The eye hit-target, the label fill, and the thumbnail/name
hit-targets SHALL all mirror this layout.

#### Scenario: The eye is centred in its gutter [lpr_eye_gutter]

- **WHEN** a row is shown at any depth
- **THEN** the eye glyph's own rect has equal left and right padding inside the
  fixed visibility gutter

#### Scenario: A separator divides the gutter from the content [lpr_eye_sep]

- **WHEN** a layer row is shown
- **THEN** a 1 px darker-grey vertical separator is drawn at the gutter's right
  edge between the eye and the content

#### Scenario: A non-group thumbnail sits close to the gutter [lpr_thumb_gap]

- **WHEN** a regular non-expandable layer and a group row are shown at the same
  depth
- **THEN** the regular layer's thumbnail starts immediately after the gutter,
  while the group row still reserves the chevron slot for its disclosure icon

### Requirement: Layer thumbnail preserves the canvas aspect ratio

A layer row thumbnail SHALL be drawn at the document's aspect ratio inside the
row's thumbnail box, letterboxed rather than stretched into a square, so a wide
or tall document is not distorted. The thumbnail image itself SHALL also carry
the document's aspect ratio: a thumbnail built for Entire Document placement
SHALL size its output from the document aspect (`size` on the document's long
edge and the proportional length on the short edge), placing the layer's content
at its document-relative position, so a full-width layer has no transparent
padding strip and is not squashed. A shape layer's thumbnail SHALL likewise
carry the document aspect. The thumbnail outline, the transparency checkerboard,
the active bracket, and the thumbnail hit-target SHALL all use the letterboxed
rect. A group's folder glyph keeps its square slot.

#### Scenario: A non-square document is not stretched [lpr_thumb_aspect]

- **WHEN** a regular layer of a document whose width and height differ is shown
- **THEN** the drawn thumbnail's aspect ratio matches the document, with the
  unused part of the row's thumbnail box left empty

#### Scenario: A full-width layer has no transparent strip [lpr_thumb_source_aspect]

- **WHEN** a layer spanning the full width of a tall document is shown with
  Entire Document thumbnail contents
- **THEN** the thumbnail image has the document's aspect ratio and its rightmost
  column is layer content rather than transparent padding

### Requirement: Layer drag shows a CS6 drop indicator

While a layer drag is over a valid target, the view SHALL draw the CS6 drop
indicator itself rather than the stock style primitive: a thin blue line above
or below the target row for a sibling reorder, and a thin blue outline around a
group row for a drop-into. An invalid target SHALL continue to show no
indicator, and the closed-hand cursor and the above/below resolution SHALL be
unchanged.

#### Scenario: A sibling reorder shows a thin blue line [lpr_drop_line]

- **WHEN** a row is dragged over the gap above or below a valid sibling target
- **THEN** a thin blue line is drawn at that edge and no other indicator is shown

#### Scenario: A drop into a group outlines the group [lpr_drop_group_outline]

- **WHEN** a row is dragged over a group row that is a valid target
- **THEN** a thin blue outline is drawn around the group row

#### Scenario: An invalid target shows no indicator [lpr_drop_line_invalid]

- **WHEN** a row is dragged over its own descendant or another refused target
- **THEN** no drop indicator is drawn

### Requirement: Raised layer row height

The single named row-height constant SHALL be raised so a Medium-thumbnail row
is at least 34 px tall, giving layer rows slightly more vertical breathing room
while keeping the same constant used by both `sizeHint` and the delegate's
centring math. The existing "at least 28 px" floor remains satisfied.

#### Scenario: A Medium row is taller [lpr_row_height_raised]

- **WHEN** a row's size hint is read with the Medium thumbnail size
- **THEN** its height is at least 34 px from the one named row-height constant

### Requirement: Reparent into a group resolves post-removal indices

A layer drag SHALL be able to reparent a layer into a group and out of a group
regardless of the dragged layer's position relative to the target. The
destination parent SHALL be resolved in post-removal coordinates, so a layer that
sits above the target group is not mis-resolved after its own removal. The move
SHALL insert the layer as the last child of the group and keep every other row at
its original position; a successful move is one undoable step. All existing
refusals SHALL remain: the Background, a fully- or nesting-locked source, a drop
onto the source or into its own descendant, and an Into target that is not a
group.

#### Scenario: A layer above a group can be dropped into it [lpr_drag_into_above]

- **WHEN** the root has `[A, G]` (G a group) and A is dragged onto G
- **THEN** A becomes G's last child and the move succeeds as one undo step

#### Scenario: A layer above a group with siblings below does not mis-nest [lpr_drag_into_above_siblings]

- **WHEN** the root has `[A, G, B]` and A is dragged onto G
- **THEN** A becomes G's child, A no longer appears at the root, and B stays at
  the root in its original position

#### Scenario: A group child can be moved out of its group [lpr_drag_out]

- **WHEN** a child of a group is dragged to the empty viewport below the last row
  or above/below a root sibling
- **THEN** it is reparented to the root (or the sibling's container) in one undo
  step and no other row changes position

#### Scenario: Invalid reparents are still refused [lpr_drag_into_refused]

- **WHEN** a layer is dropped into its own descendant, onto itself, or a
  Background/locked source is dragged
- **THEN** the move is refused and the document is unchanged

### Requirement: Layers property control row layout

The Layers panel SHALL lay out the blend-mode selection and the Opacity control
in one row that fractions the available slack between them, with the blend-mode
input taking the larger share and neither control dominating the row or pushing
the other out. The row SHALL stay usable at the panel's shared minimum width.

#### Scenario: The blend and opacity controls share the row [wpx_blend_row]

- **WHEN** the Layers panel is laid out at its normal width
- **THEN** the blend-mode input and the Opacity field each keep a usable width,
  and the blend-mode input does not absorb the whole row

### Requirement: Smart Filters tree rows

When a smart-object layer carries one or more smart filters, the Layers panel SHALL show a `Smart Filters` parent row directly under that layer, with one child row per filter carrying that filter's name. The parent row SHALL expose a visibility toggle for the group enable flag and each child row SHALL expose a visibility toggle for that filter's enable flag. Toggling either SHALL update the composite, record one history state, and persist the flag to the layer's preserved smart-object descriptor so it survives a write and re-read; the synthetic rows SHALL be non-editable and non-draggable and SHALL NOT appear as selectable layers to layer operations.

#### Scenario: The parent and one child per filter appear

- **WHEN** a smart-object layer with a filterFX chain is present
- **THEN** the panel shows a `Smart Filters` parent with one child row per filter, named for each filter

#### Scenario: Toggling a filter eye updates the composite

- **WHEN** a child row's visibility toggle is flipped
- **THEN** that filter's enable flag flips, the composite updates, and one history state is recorded

#### Scenario: Toggling the group eye bypasses the chain

- **WHEN** the parent row's visibility toggle is flipped off
- **THEN** the group enable flag is cleared and the composite falls back to the unfiltered source

#### Scenario: A toggle survives a write and re-read

- **WHEN** a filtered document whose filter or group visibility was toggled is written and read back
- **THEN** the re-read smart object reports the toggled flag

#### Scenario: Synthetic rows are not real layers

- **WHEN** a smart-filter row is double-clicked to rename or dragged
- **THEN** it is not renamed or moved and no layer operation targets it

### Requirement: Layer row surface and two-column layout

Each layer row's background SHALL be one shade step lighter than the list/base
background. Each row SHALL be split into two columns — a fixed visibility-toggle
column and a content column holding the rest of the row — so a click in the
visibility column toggles visibility and never selects or highlights the row. The
selected-row highlight SHALL be a lighter grey painted directly by the row
delegate rather than the palette's blue highlight, and the delegate SHALL be the
single source of that selection colour with no scoped stylesheet selection rule
for the tree view. The content column SHALL carry a 2 px left padding so the
thumbnail and name are not flush against the visibility gutter. The visibility
column's label fill and its 1 px gutter separator SHALL remain unchanged by
selection, and the gutter separator SHALL stay 1 px.

#### Scenario: The row background steps lighter than the list [lpr_row_surface]

- **WHEN** a layer row is shown against the list background
- **THEN** the row background is one shade step lighter than the list background

#### Scenario: The visibility toggle cannot be highlighted [lpr_eye_no_select]

- **WHEN** a click lands in the row's visibility-toggle column
- **THEN** visibility toggles and the row is neither selected nor highlighted

#### Scenario: Selection uses a lighter grey [lpr_selection_grey]

- **WHEN** one or more layer rows are selected
- **THEN** the row delegate paints the selected-row background as a lighter grey
  rather than the blue highlight, with no separate stylesheet rule overriding it

#### Scenario: The content column has a 2 px left padding [lpr_content_padding]

- **WHEN** a layer row is shown
- **THEN** the content column begins 2 px to the right of the visibility gutter,
  with the gutter separator still 1 px

### Requirement: Layer mask row indicators

When a layer row carries a layer mask, the delegate SHALL draw a link glyph
between the layer thumbnail and the mask thumbnail when that mask is linked to
its layer, and SHALL draw a red cross over the mask thumbnail when the mask is
disabled. When a row carries a vector mask, the delegate SHALL likewise draw a
vector-mask thumbnail with its own link glyph when linked and red cross when
disabled. Clicking a mask's link glyph SHALL toggle that mask's linked state,
and `Shift`-clicking a mask thumbnail SHALL toggle that mask's enabled state,
through the matching bridge; a `Shift`-click SHALL NOT end a mask solo view on
that mask (the solo keeps showing the mask's stored coverage); those clicks SHALL
be consumed so they do not start a rename, select another row, or begin a drag. A plain click on a layer mask
thumbnail SHALL make that mask the view's mask edit target through the bridge,
and a plain click on the layer thumbnail SHALL clear the target, so the mask
thumbnail of the targeted row SHALL draw a focus border. An `Alt`/`Option`-click
on a layer mask thumbnail SHALL toggle the mask solo view (see
`ui/mask-solo-view`): it SHALL select the clicked row (activating its layer),
make that mask the mask edit target so the focus border stays on it, and toggle
the solo display, recording no history state. It SHALL be consumed so it starts
no rename, selects no *other* row, and begins no drag. `Alt`+`Shift`-clicking a
mask thumbnail SHALL be consumed without toggling the mask's enabled state or
the solo view: that combination is reserved for the rubylith overlay, which is
not implemented here. The model SHALL expose the per-row mask and vector-mask
linked and disabled states through roles populated from the bridge.

#### Scenario: A linked mask shows the link glyph [lmk_row_link]

- **WHEN** a row whose mask is linked is shown
- **THEN** a link glyph is drawn between the layer and mask thumbnails

#### Scenario: A disabled mask shows a red cross [lmk_row_disabled]

- **WHEN** a row whose mask is disabled is shown
- **THEN** a red cross is drawn over the mask thumbnail

#### Scenario: Clicking the link glyph unlinks the mask [lmk_row_link_click]

- **WHEN** the user clicks the link glyph on a linked mask row
- **THEN** the mask is unlinked in one undoable step

#### Scenario: Shift-clicking the mask thumbnail disables it [lmk_row_shift]

- **WHEN** the user `Shift`-clicks the mask thumbnail of an enabled mask
- **THEN** the mask is disabled in one undoable step

#### Scenario: Shift-clicking a soloed mask keeps it soloed [lmk_row_shift_solo]

- **WHEN** a mask is soloed and the user `Shift`-clicks its thumbnail to toggle
  its enabled state
- **THEN** the mask's enabled state toggles and the canvas still shows the mask
  plane

#### Scenario: Selecting another layer ends the solo [lmk_row_solo_exit]

- **WHEN** a mask is soloed and the user selects a different layer row
- **THEN** the solo view ends and the canvas shows the composite

#### Scenario: A plain click activates the mask [lmk_row_activate]

- **WHEN** the user clicks the mask thumbnail of a row that carries a mask
- **THEN** that mask becomes the mask edit target and the thumbnail draws a focus
  border, with no history state

#### Scenario: A layer-thumbnail click deactivates the mask [lmk_row_deactivate]

- **WHEN** a mask is the edit target and the user clicks that row's layer
  thumbnail
- **THEN** the mask edit target is cleared and the focus border disappears

#### Scenario: Alt-clicking the mask thumbnail toggles solo [lmk_row_alt_solo]

- **WHEN** the user `Alt`-clicks the mask thumbnail of a raster-mask row
- **THEN** the row is selected, the mask becomes the edit target with its focus
  border, and the canvas shows the mask plane, with no history state

#### Scenario: A second Alt-click restores the composite [lmk_row_alt_restore]

- **WHEN** the user `Alt`-clicks the same mask thumbnail again while it is soloed
- **THEN** the composite is shown again and the mask stays the edit target

#### Scenario: Alt+Shift is reserved for rubylith [lmk_row_alt_shift]

- **WHEN** the user `Alt`+`Shift`-clicks a mask thumbnail
- **THEN** neither the solo view nor the enabled-state toggle takes effect

#### Scenario: A vector mask shows its own indicator [vmk_row_vector]

- **WHEN** a row whose vector mask is linked and enabled is shown
- **THEN** a vector-mask thumbnail is drawn alongside the layer thumbnail with
  its link glyph

#### Scenario: A disabled vector mask shows a red cross [vmk_row_vector_disabled]

- **WHEN** a row whose vector mask is disabled is shown
- **THEN** a red cross is drawn over the vector-mask thumbnail

#### Scenario: Clicking the vector link glyph unlinks it [vmk_row_vector_link_click]

- **WHEN** the user clicks the vector-mask link glyph on a linked row
- **THEN** the vector mask is unlinked in one undoable step

#### Scenario: Shift-clicking the vector thumbnail disables it [vmk_row_vector_shift]

- **WHEN** the user `Shift`-clicks the vector-mask thumbnail of an enabled vector mask
- **THEN** the vector mask is disabled in one undoable step

### Requirement: Fill / Adjustment creation menu

The Layers panel's New Fill / Adjustment strip button SHALL open a menu whose
Fill group offers Solid Color…, Gradient…, and Pattern…. Solid Color and
Gradient SHALL create the matching fill layer through the bridge. Pattern SHALL
be enabled when the open document contains at least one pattern resource and
SHALL be disabled with a "No patterns are loaded" reason when it contains none.
The Adjustment group SHALL list all sixteen CS6 adjustment kinds. Choosing an
adjustment kind SHALL create that adjustment layer through the bridge. The Fill
and Adjustment groups SHALL be separated as in CS6. The `Layer > New Fill Layer`
submenu SHALL offer the same three fill types with the same Pattern gating.

#### Scenario: The menu lists every adjustment kind

- **WHEN** the New Fill / Adjustment menu is opened
- **THEN** it contains an entry for each of the sixteen adjustment kinds, each enabled, and the choosing creates the matching layer

#### Scenario: Pattern stays disabled until authoring exists

- **WHEN** the New Fill / Adjustment menu is opened on a document with no pattern resources
- **THEN** the Pattern… entry is present, disabled, and carries a no-patterns reason

#### Scenario: Pattern follows the document's pattern resources [lpr_pattern_entry]

- **WHEN** the menu is opened on a document with patterns and on one without
- **THEN** Pattern… is enabled on the first and disabled with the no-patterns reason on the second

#### Scenario: The Layer menu offers the fill types [lpr_layer_fill_menu]

- **WHEN** the `Layer > New Fill Layer` submenu is built
- **THEN** it offers Solid Color…, Gradient…, and Pattern… with the same enablement as the panel menu

### Requirement: Layer Content Options opens the fill / adjustment editor

`Layer > Layer Content Options…` SHALL be enabled only when the current layer is
a fill or adjustment layer. Invoking it on a fill layer SHALL open that fill
type's editor; invoking it on an adjustment layer SHALL show the Properties
panel and refresh it so it reflects that layer. For any other current layer the
command SHALL be disabled and invoking it SHALL do nothing.

#### Scenario: The command opens the fill editor

- **WHEN** a Solid Color, Gradient, or Pattern fill layer is current and the command is invoked
- **THEN** the matching fill editor opens on that layer

#### Scenario: The command opens Properties for an adjustment layer

- **WHEN** an adjustment layer is current and `Layer Content Options…` is invoked
- **THEN** the Properties panel is shown and reflects that layer

#### Scenario: The command is disabled for other layers

- **WHEN** the current layer is a pixel, type, group, or Background layer
- **THEN** `Layer Content Options…` is disabled and invoking it does nothing

### Requirement: Type and shape forced locks

The Layers panel lock strip SHALL show Lock Transparency and Lock Image forced on
for a type or shape layer, and SHALL NOT allow either to be deselected. The two
toggle buttons SHALL be disabled while such a layer is the current row so a click
cannot clear them, and the layer's reported lock state SHALL include those two
bits so the strip renders them checked. The engine SHALL enforce the same rule:
a request that would clear the Transparency or Image lock on a type or shape
layer SHALL leave those bits set (clearing any other requested bit such as
Position still succeeds) and SHALL record no history state when nothing changed.

#### Scenario: A shape layer shows the forced locks

- **WHEN** a shape layer becomes the current row
- **THEN** the Lock Transparency and Lock Image toggles are checked and are
  disabled so they cannot be deselected

#### Scenario: A type layer shows the forced locks

- **WHEN** a type layer becomes the current row
- **THEN** the Lock Transparency and Lock Image toggles are checked and are
  disabled so they cannot be deselected

#### Scenario: The bridge refuses to clear a forced bit

- **WHEN** the lock bridge is asked to clear Transparency or Image on a type or
  shape layer
- **THEN** the bit stays set, no history state is recorded, and the layer is
  otherwise unchanged

### Requirement: Vector mask commands

The system SHALL provide `Layer ▸ Vector Mask` leaves for Reveal All, Hide All,
Current Path, Delete, Enable, Disable, Link, and Unlink, and a
`Layer ▸ Rasterize ▸ Vector Mask` leaf, each registered as implemented with a
frozen command id and wired to the vector-mask bridge over the active layer,
each applied action one undoable step. Add-style leaves SHALL be enabled when a
document exists and the active layer has no vector mask; Current Path SHALL also
require a non-empty work path; Delete, Enable, Disable, Link, Unlink, and
Rasterize SHALL be enabled only when the active layer has a vector mask.

#### Scenario: The add leaves light up without a vector mask [vmk_menu_add]

- **WHEN** a document's active layer has no vector mask
- **THEN** Reveal All and Hide All are enabled and Delete, Enable, Disable, Link,
  Unlink, and Rasterize are disabled

#### Scenario: The mask leaves light up with a vector mask [vmk_menu_present]

- **WHEN** the active layer carries a vector mask
- **THEN** Delete, Enable, Disable, Link, Unlink, and Rasterize are enabled and
  Reveal All and Hide All are disabled

#### Scenario: Current Path needs a work path [vmk_menu_current]

- **WHEN** the document has no work path
- **THEN** Current Path is disabled

#### Scenario: Reveal All adds and Delete removes through the menu [vmk_menu_dispatch]

- **WHEN** the user dispatches Reveal All and then Delete for the active layer
- **THEN** a vector mask is added and then removed, each in one undoable step

#### Scenario: Rasterize consumes the vector mask [vmk_menu_rasterize]

- **WHEN** the user dispatches Rasterize Vector Mask for a layer with a vector mask
- **THEN** the layer carries a layer mask instead and one undoable step is recorded

### Requirement: Smart Filter layer-menu commands

The Layers menu SHALL expose `Layer > Smart Filter > Clear Smart Filters` with
the stable id `layer.smartFilters.clear`, alongside the Disable Filter Mask and
Delete Filter Mask leaves. Clear Smart Filters SHALL be enabled only when the
current layer carries at least one smart filter, and dispatching it SHALL clear
the current smart object's filter stack through the document operations. The
Disable Filter Mask and Delete Filter Mask leaves SHALL remain disabled until
filter-mask decoding lands.

#### Scenario: Clear is enabled only for a smart object with filters

- **WHEN** the current layer is a smart object with one or more smart filters
- **THEN** Clear Smart Filters is enabled, and it is disabled for a non-smart layer or a smart object with no filters

#### Scenario: Dispatching Clear empties the current stack

- **WHEN** Clear Smart Filters is dispatched on the current smart object
- **THEN** the layer reports no smart filters and the panel's Smart Filters row disappears

#### Scenario: The mask leaves stay disabled

- **WHEN** the current layer is a smart object with filters but no decoded filter mask
- **THEN** Disable Filter Mask and Delete Filter Mask are disabled

### Requirement: Layer style panel integration

The Layers panel SHALL be the surface that reaches the layer-style feature. A
row double-click outside the name, the eye/mask controls, and a fill or
adjustment layer's content thumbnail SHALL open the Layer Style dialog on
Blending Options for that layer when the layer can carry a style
(`layer_style_can_edit`), and SHALL do nothing otherwise. A double-click on an
adjustment layer's content thumbnail SHALL show the Properties panel; a
double-click on a fill layer's content thumbnail SHALL open its fill editor. The
row context menu SHALL offer **Blending Options…**, **Copy Layer Style**,
**Paste Layer Style**, and **Clear Layer Style** as implemented commands:
Blending Options SHALL appear for pixel, group, and type rows and SHALL be
enabled only where the layer can carry a style; Copy and Clear SHALL be enabled
when the row's layer carries a style; Paste SHALL be enabled when the app-wide
style clipboard is non-empty. Choosing a command SHALL act on the row's layer
through the layer-style bridge and SHALL record through the bridge's own
history. The Background's brief row menu SHALL NOT offer Blending Options or the
style clipboard rows.

An `Alt`-click on a styled-or-adjustment row's right-edge `fx` region SHALL
toggle every layer's effects through `layer_style_set_all_visible`, showing
them when all were hidden and hiding them when any were shown, as one undoable
step.

The filter row's **Effect** dimension SHALL be enabled and SHALL list the ten
effect keys from `layer_style_effect_names()`; selecting an effect SHALL match
the rows whose layer carries that effect (and, as with the other dimensions,
their ancestors), and a row whose layer does not carry it SHALL be filtered out.

#### Scenario: A double-click opens Blending Options

- **WHEN** a pixel layer row is double-clicked outside its name and its eye/mask controls
- **THEN** the Layer Style dialog opens on Blending Options for that layer

#### Scenario: A background double-click does not open a style

- **WHEN** the Background row is double-clicked outside its name
- **THEN** the Background conversion path runs and no Layer Style dialog opens

#### Scenario: The style rows report their enablement

- **WHEN** the row menu is opened for a pixel layer that carries a color overlay and has a copied style available
- **THEN** Blending Options, Copy Layer Style, Paste Layer Style, and Clear Layer Style are all enabled

#### Scenario: An unstyled pixel row disables Copy and Clear

- **WHEN** the row menu is opened for a pixel layer with no layer style
- **THEN** Blending Options is enabled and Copy Layer Style, Paste Layer Style, and Clear Layer Style are disabled

#### Scenario: An adjustment row omits the style rows

- **WHEN** the row menu is opened for an adjustment layer
- **THEN** it does not offer Blending Options, Copy Layer Style, Paste Layer Style, or Clear Layer Style

#### Scenario: The row menu clears a row's style

- **WHEN** Clear Layer Style is chosen for a pixel row that carries a style
- **THEN** the row's style is removed in one undoable step and its fx badge disappears

#### Scenario: Alt-clicking the fx region toggles all effects

- **WHEN** the user `Alt`-clicks the fx badge of a styled row while effects are shown
- **THEN** every layer's effects are hidden in one undoable step, and a second `Alt`-click shows them again

#### Scenario: The Effect dimension matches styled rows

- **WHEN** the filter uses the Effect dimension with an effect a layer carries
- **THEN** that layer's row is shown and rows without the effect are hidden

#### Scenario: The Effect dimension lists the effect keys

- **WHEN** the filter's Effect dimension is selected
- **THEN** its choices are the ten effect keys from the layer-style effect list

#### Scenario: An adjustment content thumbnail opens Properties [lpr_adjustment_thumb_opens]

- **WHEN** the user double-clicks an adjustment layer's content thumbnail
- **THEN** the Properties panel is shown on that adjustment and no Layer Style dialog opens

#### Scenario: A fill content thumbnail opens the fill editor [lpr_fill_thumb_opens]

- **WHEN** the user double-clicks a fill layer's content thumbnail
- **THEN** the matching fill editor opens and no Layer Style dialog opens

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
thumbnails SHALL have the same aspect ratio as the image thumbnail. The chain
glyph SHALL be drawn two pixels larger than its slot and rotated a quarter turn
so it reads vertically between the side-by-side thumbnails, in both the linked
and unlinked states. A row's active edit target SHALL be marked with brackets
around that thumbnail, and clicking a thumbnail SHALL make it the active target;
Alt+clicking a thumbnail SHALL also activate it in the composite. A group row
SHALL NOT draw brackets around its icon. An **adjustment row's image thumbnail
SHALL be a neutral card bearing the icon for its decoded adjustment kind**, with
no checkerboard; a **fill row's image thumbnail SHALL show its content preview**
(a flat solid swatch, a gradient ramp, or a scaled pattern tile) in the same
letterboxed geometry as a pixel thumbnail.

#### Scenario: Mask thumbnail adjacency [uip_layers_mask_adjacency]

- **WHEN** a layer with a mask and a vector mask is shown
- **THEN** the mask thumbnail is directly right of the image thumbnail and the vector mask directly right of the mask, each with a chain glyph

#### Scenario: Active thumbnail brackets [uip_layers_active_thumb]

- **WHEN** a layer thumbnail is clicked to become the edit target
- **THEN** brackets mark that thumbnail and a group row never shows brackets on its icon

#### Scenario: The chain glyph is larger and rotated [uip_layers_link_glyph]

- **WHEN** a masked or vector-masked row is shown, linked or unlinked
- **THEN** its chain glyph is drawn two pixels larger than the slot and rotated
  a quarter turn, so its ink is taller than it is wide

#### Scenario: An adjustment row shows its kind icon [lpr_thumb_adjustment_icon]

- **WHEN** an Invert, Levels, or other adjustment layer is shown
- **THEN** its thumbnail is a neutral card with that adjustment kind's icon and no checkerboard

#### Scenario: A fill row shows its content preview [lpr_thumb_fill_preview]

- **WHEN** a Solid Color, Gradient, or Pattern fill layer is shown
- **THEN** its thumbnail shows the fill's solid colour, gradient ramp, or scaled pattern tile instead of a generic checkerboard

### Requirement: Layers panel folder rows

A folder row SHALL be vertically shorter than a layer row, its folder icon SHALL
be about three quarters of the row thumbnail size, and its right column SHALL use
less left-side padding without changing child-layer visibility controls. A group
row SHALL show its expand/collapse chevron even when it has no children.

#### Scenario: Empty group is expandable [uip_layers_folder_chevron]

- **WHEN** a group with no children is shown
- **THEN** the row shows its chevron and can be expanded

#### Scenario: The folder glyph is near thumbnail size [uip_layers_folder_icon]

- **WHEN** a group row is shown
- **THEN** its folder glyph is about three quarters of the row thumbnail size,
  larger than half

### Requirement: Layers panel visibility toggle responsiveness

Toggling a layer's visibility SHALL take effect immediately and SHALL NOT block a
second toggle while the model refreshes.

#### Scenario: Two quick visibility toggles [uip_layers_visibility_responsive]

- **WHEN** a layer's eye is clicked twice in quick succession
- **THEN** both toggles apply

### Requirement: A Background row offers Convert to Smart Object

The Layers-panel row context menu SHALL offer **Convert to Smart Object** on a
Background row, in addition to a pixel row, wired to the convert bridge and
acting on that row. It SHALL be enabled only when the row is a convertible raster
pixel or Background layer. A successful conversion SHALL refresh the panel and
select the row, which is no longer a Background.

#### Scenario: A Background row offers the command

- **WHEN** the user opens the row context menu for the Background layer
- **THEN** it contains Convert to Smart Object, enabled

#### Scenario: A group row does not offer the command

- **WHEN** the user opens the row context menu for a group layer
- **THEN** it does not contain Convert to Smart Object

#### Scenario: The Background row converts and reselects

- **WHEN** the user picks Convert to Smart Object on the Background row
- **THEN** the row becomes a smart object, is no longer a Background, one undo state is recorded, and the panel reselects the row

### Requirement: The Background's lock badge unlocks it

Clicking the lock badge on the Background row SHALL convert the Background into an ordinary layer at once, without a dialog: it SHALL be named with the next free `Layer N` name, unflagged, unlocked, and given an opaque alpha channel, as one undoable step. Converting a Background by any path SHALL give it an alpha channel so it can take transparency.

#### Scenario: Clicking the Background's lock unlocks it

- **WHEN** the lock badge on an opened photo's Background row is clicked
- **THEN** the row becomes an unlocked pixel layer and exactly one history state is recorded

### Requirement: Active layer row reflects the document

The system SHALL keep the document's layer target reflected in the panel. The
panel SHALL write the user's selection into the document target when the user
changes it, and SHALL use the current row as the active path when the current
row is part of that selection. When the current row is outside the selection,
the panel SHALL leave the target's active path unchanged. On open, on a
document switch, and on any refresh, the panel SHALL select the rows named by
the document target and SHALL NOT capture or overwrite that target; it SHALL
NOT fall back to the first displayed row. When the target's active path moves to another layer and that
layer's row is displayed, the panel's current row SHALL follow it, so that the
current row is the active layer. A current row the user moved outside the
selection SHALL survive a refresh that leaves the target unchanged while the row
is still displayed. An explicit deselect (Select > Deselect Layers, or a click
on empty space) SHALL clear the document target and SHALL NOT be undone by the
next refresh. A filter SHALL hide rows without changing the target; when the
filter is cleared, the target's visible rows SHALL be selected again. A
selection-based command SHALL use the target even while a selected row is
hidden by the filter. With several layers selected, every selected row SHALL
remain visually selected. A document whose only layer is the locked
`Background` SHALL report that row as active.

#### Scenario: The Background is active when it is the only layer [lpr_bg_active]

- **WHEN** a document whose only layer is the locked Background is opened or refreshed
- **THEN** the Background row is the selected and active row, and the document's
  active-layer path is its path

#### Scenario: The active layer is restored when it is not the first row [lpr_active_restore]

- **WHEN** a refresh rebuilds the model and the document's target is not the
  first displayed row
- **THEN** the panel selects the target's path rather than the first displayed
  row

#### Scenario: The current row follows the target's active path [lpr_active_follows]

- **WHEN** an operation moves the document target's active path to another
  layer and that layer's row is displayed
- **THEN** the panel's current row is the new active path

#### Scenario: A current row outside the selection survives a refresh [lpr_active_stray]

- **WHEN** the user moves the panel's current row outside a multi-selection
  whose active path is not the first selected path, and the panel refreshes
  while the document target stays the same and the row is still displayed
- **THEN** the current row stays on the row the user put it on and the
  document's active path stays on the previously active selected layer

#### Scenario: A document switch shows the new document's active row [lpr_switch_current]

- **WHEN** the user switches to another open document
- **THEN** the panel's selected rows are that document's target and its current
  row is that document's active path

#### Scenario: An explicit deselect is preserved [lpr_active_deselect]

- **WHEN** the user clears the layer selection, leaving the document with no
  active layer
- **THEN** the next refresh leaves the panel with no selected row

#### Scenario: A filter hiding the target does not clear it [lpr_filter_target]

- **WHEN** an active filter hides the row named by the document target
- **THEN** the document still reports that layer as selected and active, and
  clearing the filter selects its row again

#### Scenario: A command uses a filtered-out target [lpr_filter_command]

- **WHEN** the user runs a selection-based layer command while a selected row is
  hidden by the filter
- **THEN** the command acts on that hidden selected layer as part of the target

#### Scenario: A multi-selection is shown in full [lpr_multi_visual]

- **WHEN** the document target holds several selected layers
- **THEN** every visible selected row is shown as selected and the active path's
  row is the current row

### Requirement: Drag a layer into another document

A Layers-panel drag SHALL carry the document it started from. A Move-tool
drag on the canvas with no selection SHALL become such a drag of the active
layer once the pointer reaches another document's tab, leaving the source
layer where it was. While dragging,
hovering another open document's tab SHALL make that document active, and
releasing the drag on that tab or on its canvas SHALL deep-copy the dragged
layer (children, masks, effects, and attributes included) into it. The copy
SHALL be placed by the New Layer insertion rule relative to the destination's
current layer, SHALL keep its name and document position, and SHALL become the
selected row, all as one undoable "Duplicate Layer" step. The source document
SHALL be unchanged. A copy of the Background SHALL be an ordinary, unlocked
layer. A drop on the source document's own tab or canvas SHALL copy nothing. A
destination of a different color mode or bit depth SHALL refuse the drop,
leaving both documents unchanged. The Layers panel SHALL NOT treat a drag from
another document as a drop on its rows or strip buttons.

#### Scenario: Drop a layer on another document's tab [lpr_drag_to_document]

- **WHEN** layer `Sky` is dragged from one document onto another document's tab and released
- **THEN** that document becomes active and gains a selected layer `Sky` with
  the same opacity in one undo step, and the source keeps its layers

#### Scenario: Move-tool drag from the canvas onto another document's tab

- **WHEN** the Move tool drags from one document's canvas onto another document's tab and releases
- **THEN** that document becomes active and gains a copy of the active layer, and the source layer has not moved

#### Scenario: Drop a layer on another document's canvas

- **WHEN** a layer is dragged from one document and released on the canvas of another, active document
- **THEN** that document gains a copy of the layer

#### Scenario: A mode mismatch refuses the drop

- **WHEN** a layer of an RGB document is released on a Grayscale document's tab
- **THEN** the Grayscale document is unchanged

#### Scenario: The destination panel ignores the foreign drag

- **WHEN** a drag from one document is released on the Layers panel rows after another document became active
- **THEN** the drop is refused and the active document is unchanged

### Requirement: Layer row drop bands

During a layer drag the Layers panel SHALL resolve the release point on a row by
the row's kind. A row that is not a group SHALL be a sibling target across its
whole height: a release in its upper half SHALL drop above it and a release in
its lower half SHALL drop below it, and it SHALL NOT resolve to a drop into the
row. A group row SHALL resolve a release within its upper quarter (at least
2 px) to above, within its lower quarter (at least 2 px) to below, and anywhere
between to a drop into the group.

#### Scenario: A release inside a layer row reorders [lpr_drop_row_halves]

- **WHEN** layer A is dragged and released a quarter of the way down layer B's row
- **THEN** A moves directly above B in one undo step

#### Scenario: The lower half of a layer row drops below it

- **WHEN** a layer is dragged and released three quarters of the way down another layer's row
- **THEN** it moves directly below that layer

#### Scenario: The centre of a group row still drops into it

- **WHEN** a layer is dragged and released at the vertical centre of a group row
- **THEN** it becomes a child of that group

### Requirement: Layers panel combo affordances

The Layers panel's blend-mode and layer-filter combo boxes SHALL draw a
drop-down caret — a small down-pointing triangle in the UI text colour, matching
the design system's `.combo .caret` — so their selectable nature is visible, and
SHALL cycle their options with the Up and Down arrow keys while the field holds
keyboard focus. A value change SHALL NOT drop the field's focus, so repeated
arrow keys keep cycling until focus moves elsewhere.

#### Scenario: A combo shows a drop-down caret [lpr_combo_arrow]

- **WHEN** a Layers panel combo box is shown
- **THEN** it draws a small down-pointing caret at its right edge

#### Scenario: Arrow keys cycle the focused combo [lpr_combo_arrows]

- **WHEN** a Layers panel combo box has keyboard focus and Down is pressed
- **THEN** its current option advances by one, and Up moves it back

#### Scenario: The field keeps focus across a change [lpr_combo_focus]

- **WHEN** a focused combo changes its value (its blend edit refreshes the panel)
- **THEN** the field still holds focus and the next arrow key cycles it again

### Requirement: Group expansion survives a structural change

A group's expanded/collapsed state SHALL survive a structural change that shifts
the group's index-based path — adding, removing, or moving a layer, or dropping
a layer into the group. The panel SHALL re-find the group by its ancestor name
chain, disambiguating same-named siblings by their order among them, and keep it
expanded or collapsed as before. A rename, which leaves the index path intact,
SHALL keep the state through the unchanged path. A refused operation SHALL change
neither the document nor the expansion. The expanded groups SHALL belong to the
document they were expanded in: switching to another document SHALL NOT carry
the expansion across, and switching back SHALL restore that document's own
expanded groups.

#### Scenario: Adding a layer outside an expanded group keeps it open [lpr_add_keeps_expanded]

- **WHEN** a group is expanded and a layer is added elsewhere in the document so
  the group's index-based path shifts
- **THEN** the group stays expanded with its children visible

#### Scenario: A rename keeps the group expanded [lpr_rename_keeps_expanded]

- **WHEN** an expanded group is renamed in place
- **THEN** it stays expanded, because the rename leaves its index path intact

#### Scenario: Same-named sibling groups stay distinct [lpr_dup_group_names]

- **WHEN** two sibling groups share a name, one is expanded, and a structural
  change shifts their paths
- **THEN** the group that was expanded is the one that stays expanded

#### Scenario: A drop into an open group keeps it open [lpr_drop_into_expanded]

- **WHEN** an open group is a valid drop-into target and a layer is dropped onto it
- **THEN** the move succeeds as one undo step and the group stays expanded with
  the dropped layer visible as its child

#### Scenario: Switching documents does not carry the expansion [lpr_expansion_document]

- **WHEN** a group is expanded in one document and the user switches to another
  document
- **THEN** the other document's groups are not expanded, and switching back
  restores the first document's expanded group

### Requirement: Layers panel action strip geometry

The Layers panel's seven-button action strip SHALL keep a constant 2 px gap
between adjacent buttons and SHALL stay horizontally centred in the panel as the
panel width changes. Each strip button SHALL keep its intrinsic width. Surplus
panel width SHALL be absorbed by the space at the strip's leading and trailing
ends, not by the inter-button gaps or the buttons. The strip's button order,
object names, icons, tooltips, click behaviour, and drop targets SHALL remain
unchanged.

#### Scenario: The strip keeps its fixed gap and stays centred [lpc_strip_geometry]

- **WHEN** the Layers panel is shown and then widened while its overall width
  stays above the panel's minimum width
- **THEN** every adjacent pair of strip buttons keeps the same 2 px gap, each
  button keeps the same width it had before the resize, and the distance from
  the panel's left edge to the first button equals the distance from the last
  button to the panel's right edge within one pixel

### Requirement: Layer row badges, labels, and delegate

The system SHALL draw each row with a delegate that paints, in CS6 order, the
visibility toggle, the thumbnail (a folder glyph for a group), the name, a
clipping-mask indicator for a clipped layer, the clipping indentation and base
underline, the layer-mask thumbnail when a mask is present, an adjustment badge
when adjustment content is present, a layer-style `fx` badge when the layer
carries a layer style (an `lfx2` or legacy `lrFX` block), and a compact
**`Blend If` text chip** when the layer's advanced blending is customised — a
non-default `Blend If` view, or a raw `blending_ranges` block present without a
typed view. The chip SHALL be a text-only badge (no icon asset) drawn in the
row's right-edge badge run left of the `fx` and lock badges, and its advance
SHALL be reserved by the same right-edge walk that lays out the mask thumbnails
and the name rect so nothing overlaps. A layer whose lock
state has any flag set SHALL also show a lock badge at the right side of its
row, except that the Transparency and Image locks a type or shape layer is
forced to carry SHALL NOT by themselves show one (CS6 shows them only in the
Lock strip); an unlocked layer SHALL show none. The visibility toggle SHALL be an eye
icon (`layers.eyeOn`/`layers.eyeOff`) drawn slightly inset from the panel's left
edge and at the same x for every row, independent of nesting depth; the nesting
indentation SHALL apply to the thumbnail and name, not to the visibility toggle.
A layer whose color label is not `None` SHALL fill the **whole visibility
column** (the full 30 px column, at the row's full height) with that label
color at full opacity behind the eye glyph. The eye glyph SHALL remain legible
against the fill, a selected labeled row SHALL keep the label fill in its
visibility column rather than the selection surface, and the label color
elsewhere on the row SHALL fall back to the row background. The fill is
paint-only and SHALL NOT alter the visibility column's hit target. The labeled
row's thumbnail, name, mask, link, and vector hit rectangles SHALL match an
unlabeled row's x, with no color-label chip or chip-reserved space before the
thumbnail. The row's 1 px bottom rule and the gutter's 1 px separator SHALL
remain drawn over the fill. A group with at least one child
SHALL show a disclosure icon — right when collapsed, down when expanded — at its
indented position, and clicking that icon SHALL expand or collapse the group. A
**regular (non-group) layer's thumbnail SHALL be drawn over a cached two-tone
checkerboard** so transparency reads under it, while a group keeps its folder
glyph and no checkerboard. A **type layer's thumbnail SHALL be drawn as a
square (1:1) white card bearing a "T"**, sized to the thumbnail's side and
centred in its slot, regardless of the document's aspect ratio; it SHALL NOT be
letterboxed to the document, and its hit rectangle SHALL be that square card.
Every other non-group layer's thumbnail SHALL be laid out in a box `thumb` wide
and the row's full height minus a small pad, and SHALL be letterboxed to the
document's aspect ratio inside that box, so a tall document's thumbnail fills the
row vertically rather than floating in a square box. **Every non-group thumbnail
other than a type layer's SHALL carry a 1 px black outline**; a type layer's
card and a group's folder glyph SHALL draw no outline, and **when exactly one
layer is active** (the same singular
active-layer resolution tool edits use) its thumbnail SHALL additionally show
white 1 px corner brackets drawn one pixel outside the thumbnail card (for a
regular layer, outside the outline); with zero or
multiple active layers no brackets are drawn. A layer whose row reports the
smart-object projection SHALL paint the `layers.kindSmartObject` badge on the
thumbnail's lower-right corner, whether the smart object is embedded or
placed/external; the badge SHALL be omitted when the asset is missing. Row
typography SHALL derive from the layer: a `background` layer's name SHALL be
italic/cursive, every other name normal, and a layer that is a member of the
frame's link set or a placed/external smart object SHALL be underlined through
the `LayerRowLinkedRole` and `LayerRowPlacedRole` projections. The delegate SHALL
report a row height of at least **28 px** through one named constant used by both
`sizeHint` and the vertical centring math. If an expected icon asset is
unavailable, the delegate SHALL omit that badge while keeping the row legible
rather than fail.

#### Scenario: The visibility toggle is an eye icon [lpr_eye]

- **WHEN** a layer row is shown
- **THEN** its visibility toggle is drawn from the eye icon asset, and a hidden
  layer uses the off variant

#### Scenario: The eye is left-anchored for every depth [lpr_eye]

- **WHEN** a nested layer is shown under its group
- **THEN** its eye icon is at the same x as a top-level row's, while its
  thumbnail and name are indented

#### Scenario: The color label fills the visibility cell [lpr_label_tint]

- **WHEN** a layer with a non-`None` color label is shown
- **THEN** the entire visibility column is filled with that label color behind
  the eye glyph, the glyph stays legible against the fill, no separate chip is
  painted, and the rest of the row background is unchanged

#### Scenario: An unlabeled layer keeps the normal gutter [lpr_label_tint_none]

- **WHEN** a layer with color label `None` is shown
- **THEN** its whole visibility column uses the normal gutter surface and no
  tint or chip is painted

#### Scenario: The eye glyph stays legible on every label [lpr_label_glyph]

- **WHEN** a row carries each of the seven color labels
- **THEN** the eye glyph is drawn with a dark tint on bright fills and a light
  tint on dark fills, so it stays readable against the fill

#### Scenario: A smart object shows the badge [lpr_smart_badge]

- **WHEN** an embedded or placed smart-object layer is shown
- **THEN** its thumbnail draws the smart-object badge on the lower-right corner

#### Scenario: A non-smart layer shows no smart badge [lpr_smart_badge_absent]

- **WHEN** an ordinary pixel layer is shown
- **THEN** no smart-object badge is drawn on its thumbnail

#### Scenario: A group shows a disclosure icon [lpr_chevron]

- **WHEN** a group with children is shown collapsed and then expanded
- **THEN** it shows the right-pointing icon when collapsed and the down-pointing
  icon when expanded, and clicking the icon toggles it

#### Scenario: A masked layer shows a mask thumbnail [m39_badges]

- **WHEN** a layer carries a layer mask
- **THEN** its row shows a mask thumbnail and the mask-presence flag is set

#### Scenario: A clipped layer shows the clipping indicator [lpc_clip]

- **WHEN** a layer is clipped to the layer below it
- **THEN** its row draws the clipping-mask indicator in addition to the
  indentation and the base layer's underline

#### Scenario: A group row is expandable [m39_badges]

- **WHEN** a group has children
- **THEN** its row is marked expandable and shows an expand/collapse control

#### Scenario: A locked layer shows a right-side lock badge [lpc_lockbadge]

- **WHEN** a layer has any lock flag set
- **THEN** its row draws a lock badge on the right side, and an unlocked layer
  draws no lock badge

#### Scenario: A type layer's forced locks show no badge [lpc_forced_lock_badge]

- **WHEN** `tst_type_tools::typeLayerRowHasNoLockBadge` commits a type layer, then also locks its position
- **THEN** the layer carries the Transparency and Image locks and its row has no lock badge; with Position locked too the badge shows

#### Scenario: A regular thumbnail shows a checkerboard [lpr_thumb_checker]

- **WHEN** a regular (non-group) layer with translucent pixels is shown
- **THEN** its thumbnail is drawn over a two-tone checkerboard, while a group
  row keeps its folder glyph and draws no checkerboard

#### Scenario: Thumbnails are outlined and the active layer is bracketed [lpr_thumb_bracket]

- **WHEN** a regular layer row is shown with a 1 px black thumbnail outline
- **THEN** the singular active layer's thumbnail additionally shows white
  corner brackets one pixel outside the outline, and a multi- or zero-selection
  row shows no brackets

#### Scenario: A group icon has no outline [lpr_group_no_outline]

- **WHEN** a group row is shown
- **THEN** its folder glyph carries no 1 px black outline, while a regular
  layer's thumbnail keeps its outline

#### Scenario: A type card is square with no outline [lpr_type_thumb_card]

- **WHEN** a type layer row is shown in a document of any aspect ratio
- **THEN** its thumbnail is a square white card with a "T" whose width equals
  its height, it is not letterboxed to the document, it carries no 1 px black
  outline, and the thumbnail's hit rectangle is that same square card

#### Scenario: A tall thumbnail fills the row [lpr_thumb_fill]

- **WHEN** a portrait document's layer row is shown
- **THEN** its letterboxed thumbnail is taller than it is wide and spans the
  row's full height minus the thumbnail pad

#### Scenario: Row typography follows the layer kind [lpr_row_fonts]

- **WHEN** the rows for the Background, a linked layer, and a placed smart
  object are inspected
- **THEN** the Background name is italic/cursive, ordinary names are normal, and
  the linked and placed rows are underlined through their roles

#### Scenario: The row height has a floor [lpr_row_height]

- **WHEN** a row's size hint is read
- **THEN** its height is at least 28 px from the single named row-height
  constant

#### Scenario: A missing badge asset does not break the row [m39_badges]

- **WHEN** the adjustment badge's icon asset is missing
- **THEN** the row is still drawn and the badge is simply omitted

#### Scenario: A styled layer shows the fx badge

- **WHEN** a layer that carries a layer style is shown
- **THEN** its row draws the `layers.fx` badge at its right edge beside the lock
  and mask badges, and a layer with no style draws no fx badge

#### Scenario: A customised Blend If shows the badge [lpr_blendif_badge]

- **WHEN** a layer whose advanced blending is customised (its Blend If view is
  not the full `(0, 65535)` default, or a raw `blending_ranges` block is
  present without a typed view) is shown
- **THEN** its row draws a compact `Blend If` text chip in the right-edge badge
  run, and the name elides before the chip so no rect overlaps

#### Scenario: A default Blend If shows no badge [lpr_blendif_badge_absent]

- **WHEN** a layer with no Blend If view and no `blending_ranges` block, or one
  whose every range is the full default, is shown
- **THEN** no `Blend If` chip is drawn on its row and the name keeps the full
  right-edge run

#### Scenario: A raw blending-ranges block without a typed view still badges [lpr_blendif_raw]

- **WHEN** a layer carries a non-empty `blending_ranges` block that cannot be
  parsed into a typed Blend If view
- **THEN** its row still draws the `Blend If` chip

### Requirement: Disabled opacity and fill show the default

When no layer is selected, or no selected layer is eligible for a field, the
Layers panel SHALL display `100%` in that field and SHALL leave it disabled. The
eligibility test SHALL be over the whole selection: a field with at least one
eligible selected layer remains enabled and SHALL display the current row's
value. This covers an empty selection, the Background, a fully locked layer, a
group's Fill when no selected layer is eligible for Fill, and a synthetic Smart
Filters row. Synthetic display-only rows SHALL NOT count as eligible targets.
The Background's "neither opacity nor fill" means it offers no editable value;
the disabled control still displays the default.

#### Scenario: No selection shows the default [lpc_no_selection]

- **WHEN** the Layers panel has no selected layer
- **THEN** the Opacity and Fill fields both show `100%` and are disabled

#### Scenario: An ineligible target shows the default [lpc_ineligible_default]

- **WHEN** the Background, a fully locked layer, or a synthetic Smart Filters row
  is selected, or a group is the only selected layer and the field is Fill
- **THEN** each field without an eligible target shows `100%` and is disabled

#### Scenario: One eligible layer keeps a multi-selection field enabled [lpc_multi_eligible]

- **WHEN** a selection includes at least one layer eligible for the field
  alongside an ineligible layer
- **THEN** the field stays enabled and displays the current row's value

### Requirement: Merge result selection

After a successful Merge Down, Merge Layers, or Merge Visible, the Layers panel
SHALL select exactly the merged pixel layer. Any hidden or unselected layer that
survives the merge MUST NOT remain selected.

#### Scenario: Merge Visible selects the result over a hidden survivor

- **WHEN** Merge Visible merges the visible layers while a hidden layer survives
- **THEN** the merged pixel layer is the sole selected layer and the hidden layer
  is not selected

#### Scenario: Merge Down selects the result

- **WHEN** Merge Down replaces a layer and the layer below it
- **THEN** the merged pixel layer is the sole selected layer

#### Scenario: Merge Layers selects the result

- **WHEN** Merge Layers replaces a non-contiguous selection
- **THEN** the merged pixel layer is the sole selected layer

### Requirement: Layer menu duplication follows the panel selection

`Layer ▸ Duplicate Layer…` SHALL duplicate every layer the document target
holds, addressing each layer by its path so a layer nested inside a group is
duplicated in place. The command SHALL be enabled only when the active document
has a non-empty layer target; with an empty target it SHALL be disabled and
SHALL change nothing. Each copy SHALL sit directly above its source in the same
container, and the whole operation SHALL be one undoable `"Duplicate Layer"`
step. Every copy SHALL become part of the document target, with one of them
active, and the panel SHALL show that selection.

#### Scenario: Duplicate a nested layer from the Layer menu [lpd_nested]

- **WHEN** a layer inside a group is selected and `Layer ▸ Duplicate Layer…` is
  run
- **THEN** a copy named `"<name> copy"` appears directly above the source inside
  the same group and exactly one history state is added

#### Scenario: Duplicate a multi-selection from the Layer menu [lpd_multi]

- **WHEN** two layers are selected and `Layer ▸ Duplicate Layer…` is run
- **THEN** both layers gain a copy directly above themselves, exactly one
  history state is added, and both copies are selected with one of them active

#### Scenario: Duplicate is disabled with no selection [lpd_empty]

- **WHEN** the document has no layer in its target
- **THEN** `Layer ▸ Duplicate Layer…` is disabled and running it leaves the rows
  and history unchanged

### Requirement: CS6 row-menu follow-ups for type, fill, and group rows

The Layers row context menu SHALL expose the CS6 commands for the row's kind and
SHALL render unimplemented applicable commands disabled with a
`— not implemented yet` tooltip.

Type rows SHALL add Horizontal and Vertical orientation, None/Sharp/Crisp/Strong/Smooth
anti-aliasing, Convert to Point Text or Convert to Paragraph Text, and Warp
Text…, all wired to the type bridge and acting on the clicked row. They SHALL
also offer Create Work Path and Convert to Shape disabled, Windows LCD and
Windows anti-aliasing disabled, and Copy CSS/Copy SVG, Artboard from Layers…,
Frame from Layers…, and Convert to Frame disabled. Type rows SHALL offer Quick
Export as PNG, Export As…, and an enabled Convert to Smart Object.

Fill and adjustment rows SHALL offer Add/Delete/Enable/Disable Layer Mask,
Add/Delete/Enable/Disable Vector Mask, Create/Release Clipping Mask, Merge
Layers, Quick Export as PNG, Export As…, and Convert to Smart Object. Convert to
Smart Object SHALL be disabled for a fill or adjustment target because the
engine refuses those kinds. A fill row SHALL use the `Merge Layers` label even
for a single row.

Group rows SHALL offer Quick Export as PNG, Export As…, Add/Delete/Enable/Disable
Vector Mask, Merge Layers, and an enabled Blending Options…; Convert to Smart
Object SHALL be expressed as disabled. Group effect pages remain governed by
`ui/layer-style-dialog`.

Duplicate and Delete labels SHALL be per kind and per selection size:
`Duplicate Layer…`/`Delete Layer` for one pixel, type, or adjustment/fill row,
`Duplicate Layers…`/`Delete Layers` for a multi-selection, and
`Duplicate Group…`/`Delete Group` for a group.

#### Scenario: A type row offers its CS6 text controls

- **WHEN** the user opens the row menu for a type layer
- **THEN** Horizontal, Vertical, the five anti-aliasing rows, Convert to
  Paragraph Text, and Warp Text… are enabled, and Create Work Path, Convert to
  Shape, Copy CSS, Copy SVG, and the Windows anti-aliasing rows are disabled

#### Scenario: A fill row offers its mask and merge rows

- **WHEN** the user opens the row menu for a solid-fill row above another layer
- **THEN** Delete Layer Mask, Add Vector Mask, Create Clipping Mask, Merge
  Layers, Quick Export as PNG, Export As…, and Rasterize Layer are present, and
  Convert to Smart Object and Copy CSS are disabled

#### Scenario: A group row offers export and Blending Options

- **WHEN** the user opens the row menu for a group
- **THEN** Quick Export as PNG, Export As…, Add Vector Mask, Merge Layers, and
  Blending Options… are present and Convert to Smart Object is disabled

#### Scenario: Per-kind Duplicate and Delete labels

- **WHEN** the user opens the row menu for a pixel row, a group, and a
  multi-selection
- **THEN** the labels read Duplicate Layer…/Delete Layer, Duplicate Group…/Delete
  Group, and Duplicate Layers…/Delete Layers respectively

### Requirement: Layer effects disclosure rows

The Layers panel SHALL list a styled layer's effects nested under its row: one
synthetic **Effects** group row at the layer's depth plus one, and one synthetic
child row per effect the layer currently carries at the layer's depth plus two,
named with the effect's CS6 title in the layer-style list order (Bevel & Emboss
through Drop Shadow). The layer row SHALL expose the group through its `fx`
badge: clicking the badge SHALL expand or collapse the group. The **Effects**
group row SHALL carry a disclosure that expands or collapses the effect rows and
an eye that toggles the layer's master effect switch (`fx.visible`); each effect
row SHALL carry an eye that toggles that effect's enabled state
(`<effect>.on`), while the effect stays listed so it can be switched back on. A
newly styled layer SHALL start expanded when the panel option **Expand New
Effects** is on, and a layer with no effects SHALL show no group row. Each eye
toggle SHALL be one undoable history state. A synthetic effects row SHALL NOT be
renamed, dragged, dropped on, or made the document's layer target.

#### Scenario: A styled layer shows its effects nested [lpr_fx_rows]

- **WHEN** a pixel layer carries a drop shadow and is shown in the panel
- **THEN** its row is followed by a synthetic `Effects` group row and a `Drop
  Shadow` effect row at the next two depths

#### Scenario: The Effects eye toggles the master switch [lpr_fx_group_eye]

- **WHEN** the `Effects` group row's eye is clicked while the layer's effects are shown
- **THEN** `fx.visible` becomes false, the composite shows the layer unstyled,
  and the row's eye reads hidden

#### Scenario: An effect eye toggles that effect [lpr_fx_effect_eye]

- **WHEN** a `Drop Shadow` effect row's eye is clicked
- **THEN** `dropShadow.on` flips and the composite changes, while the effect row
  stays listed

#### Scenario: A new style starts expanded [lpr_fx_expand_new]

- **WHEN** a layer with no style gains one while **Expand New Effects** is on
- **THEN** the layer's `Effects` group and its effect rows are shown expanded

#### Scenario: An unstyled layer shows no group [lpr_fx_no_group]

- **WHEN** a layer that carries no layer style is shown
- **THEN** no `Effects` row follows it

### Requirement: Layer effects row interactions

Double-clicking a layer's `fx` badge or its **Effects** group row SHALL open the
Layer Style dialog on Blending Options for that layer; double-clicking an effect
row SHALL open it on that effect's page. Right-clicking the `fx` badge, the
**Effects** group row, or an effect row SHALL open the CS6 layer-style menu,
which SHALL offer **Disable Layer Effects** (or **Enable Layer Effects** when the
layer's effects are hidden), **Blending Options…**, the ten effect entries, the
**Copy** / **Paste** / **Clear Layer Style** commands, **Global Light…** and
**Create Layer** disabled, **Hide All Effects** (or **Show All Effects**), and
**Scale Effects…**. **Disable/Enable Layer Effects** SHALL write the layer's
`fx.visible` as one undoable state. Each effect entry SHALL open the Layer Style
dialog on that effect's page, and the menu SHALL act on the layer that owns the
effects.

#### Scenario: Double-clicking an effect row opens its page [lpr_fx_dblclick_effect]

- **WHEN** the user double-clicks a `Drop Shadow` effect row
- **THEN** the Layer Style dialog opens on the Drop Shadow page for that layer

#### Scenario: Double-clicking the fx badge opens Blending Options [lpr_fx_dblclick_badge]

- **WHEN** the user double-clicks a styled layer's `fx` badge
- **THEN** the Layer Style dialog opens on Blending Options for that layer

#### Scenario: The layer-style menu lists the CS6 commands [lpr_fx_menu]

- **WHEN** the user right-clicks the `fx` badge or an effects row of a styled layer
- **THEN** the menu offers Disable Layer Effects, Blending Options…, the ten
  effect entries, Copy/Paste/Clear Layer Style, Global Light… and Create Layer
  disabled, Hide All Effects, and Scale Effects…

#### Scenario: Disable Layer Effects hides the layer's effects [lpr_fx_disable]

- **WHEN** Disable Layer Effects is chosen for a styled layer
- **THEN** the layer's `fx.visible` becomes false, its composite is unstyled, and
  exactly one history state is recorded
