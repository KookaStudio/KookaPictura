# layer-management Specification

## Purpose
Merge Down, Merge Layers, Merge Visible, Merge Clipping Mask, Flatten Image, New Layer and Group, and Background handling.

## Requirements

### Requirement: Merge Down and Merge Layers

The system SHALL provide one selection-dependent merge command **Merge Layers**
bound to `Ctrl+E`. With more than one selected layer it SHALL perform **Merge
Layers**, and with exactly one selected layer it SHALL perform **Merge Down**.
A separate `Layer > Merge Down` command SHALL expose Merge Down directly for the
active layer. The merge SHALL validate before mutating: the merge target (the
lower layer for Merge Down, the bottommost selected layer for Merge Layers)
MUST NOT be an adjustment or fill-content layer, and Merge Down MUST have a
layer directly below the selected layer. Merge Down SHALL require exactly one
selected layer and MUST be refused when there is nothing below it. A refused
merge MUST leave the document unchanged and MUST NOT add a history state.

The command SHALL composite the inputs in stacking order through the document
compositor over the union of their content rectangles, apply each input's mask
into the resulting alpha, and replace the inputs with one pixel node. Clipping
coverage is folded into the base alpha only by `Merge Clipping Mask`; the
compositor has no clipping pass yet, so Merge Down / Merge Layers / Merge
Visible share that documented gap rather than diverging. For Merge Down the
result SHALL inherit the lower layer's name, blend mode, and opacity
*(inferred)*. For Merge Layers the result SHALL occupy the topmost selected
layer's position and its blend mode and opacity SHALL be reset to `Normal` and
`255` *(inferred)*. Sibling order and parent-group references SHALL be repaired
so the result sits where the inputs were removed. Every merge SHALL recomposite
and record exactly one undo state.

#### Scenario: Merge Down composites two layers

- **WHEN** a pixel layer sits directly above another pixel layer and Merge Down
  runs
- **THEN** the two layers are replaced by one pixel layer whose pixels equal the
  compositor's composite of the pair within the compositor tolerance, whose
  alpha is the union of their nontransparent areas, and which inherits the lower
  layer's name, blend mode, and opacity

#### Scenario: Merge Layers replaces the selection at the topmost position

- **WHEN** more than one layer is selected and `Ctrl+E` runs
- **THEN** the selected layers are replaced by one pixel layer at the topmost
  selected position whose pixels equal their composite and whose blend mode and
  opacity are `Normal` and `255`

#### Scenario: The distinct Merge Down leaf is available

- **WHEN** a pixel layer sits directly above a raster layer and `Layer > Merge
  Down` is chosen
- **THEN** the pair is merged even though only one layer is selected, and the
  change is one undo step

#### Scenario: An adjustment target is refused

- **WHEN** the bottommost selected layer is an adjustment layer and merge runs
- **THEN** the merge is refused, the document is unchanged, and no history state
  is added

#### Scenario: Merge Down needs a layer below

- **WHEN** the bottom layer of the stack is the only selected layer and Merge
  Down runs
- **THEN** the merge is refused, the document is unchanged, and no history state
  is added

#### Scenario: A merge is one undo step

- **WHEN** a merge completes and the user undoes it
- **THEN** the exact pre-merge layer tree, kinds, names, masks, and pixels are
  restored

### Requirement: Merge Visible

The system SHALL provide a `Merge Visible` command bound to `Shift+Ctrl+E`
whose inputs are exactly the currently eye-visible layers, including visibility
inherited from ancestor groups. Hidden layers MUST be skipped and left in place.
The command SHALL require at least one visible layer and the active layer MUST
be visible; otherwise it SHALL be refused and the document unchanged. The result
SHALL be a single pixel layer at the topmost visible input's position, and the
command SHALL recomposite and record exactly one undo state.

#### Scenario: A hidden layer survives Merge Visible

- **WHEN** layer A is hidden, layer B is visible, and Merge Visible runs with B
  selected
- **THEN** B's content is merged into one pixel layer and hidden A remains in
  place unchanged

#### Scenario: A hidden selection is refused

- **WHEN** the active layer is hidden and Merge Visible is chosen
- **THEN** the command is unavailable and no layer changes

### Requirement: Merge Clipping Mask

The system SHALL provide a `Merge Clipping Mask` command that collapses a
clipping group into its raster base layer. The base layer MUST be a raster
(non-adjustment, non-fill-content) layer; when the base is not raster the
command SHALL be disabled and MUST refuse without changing the document. The
collapsed result SHALL occupy the base layer's position and SHALL have the
clipped layers' coverage applied into its alpha, and the command SHALL
recomposite and record exactly one undo state.

#### Scenario: A clipping group collapses into its base

- **WHEN** a raster base has one or more clipped layers above it and Merge
  Clipping Mask runs
- **THEN** the base and its clipped layers are replaced by one raster layer at
  the base's position whose alpha reflects the clipping coverage

#### Scenario: A non-raster base is refused

- **WHEN** the clipping base is an adjustment or fill-content layer and Merge
  Clipping Mask runs
- **THEN** the command is disabled, the document is unchanged, and no history
  state is added

### Requirement: Flatten Image

The system SHALL provide a `Flatten Image` command. It SHALL discard every
hidden layer without a confirmation prompt, composite all visible layers over an
opaque **white** backdrop the size of the document, and replace the entire layer
tree with a single **Background** node covering the full document with no
transparent pixels and no alpha below full opacity. Groups, clipping, masks, and
vector data are removed by flattening. The command SHALL recomposite and record
exactly one undo state that restores the prior tree and pixels.

#### Scenario: Hidden layers are discarded and transparency becomes white

- **WHEN** a document containing hidden layers and transparent areas is
  flattened
- **THEN** exactly one Background layer remains, every pixel is opaque, and the
  areas that were transparent are white

#### Scenario: Flatten removes structure

- **WHEN** a document with a group, a clipping mask, and a layer mask is
  flattened
- **THEN** the tree is a single Background layer and the structural containers
  no longer exist

#### Scenario: Flatten is one undo step

- **WHEN** the user undoes a flatten
- **THEN** the complete prior layer tree and pixels are restored

### Requirement: New Layer and New Group dialog

The system SHALL open a modal dialog when the user `Alt`-clicks the New
Layer/New Group panel button or chooses `Layer > New > Layer…` /
`Layer > New > Group…`. The dialog SHALL collect a Name, a Color label, a blend
Mode, an Opacity, a **Fill With Mode-Neutral Color** option, and a **Use
Previous Layer to Create Clipping Mask** option, and the clipping option MUST
NOT be offered for a group. On accept the system SHALL create the layer or group
with the chosen attributes directly above the selected layer (or at the top of
the stack with no selection) as one undoable step. The mode-neutral fill SHALL
be chosen from a lookup by blend mode: white for Darken, Multiply, Color Burn,
Linear Burn, Darker Color, and Divide; black for Lighten, Screen, Color Dodge,
Linear Dodge (Add), Lighter Color, Difference, Exclusion, and Subtract; 50 %
gray for Overlay, Soft Light, Hard Light, Vivid Light, Linear Light, and Pin
Light; transparent for Normal, Dissolve, Hard Mix, Hue, Saturation, Color, and
Luminosity; and the documented default for any unlisted mode SHALL be
transparent *(inferred)*.

#### Scenario: The dialog creates a configured layer

- **WHEN** the user Alt-clicks New Layer, enters a name, sets Multiply, 50 %
  opacity, and enables the neutral fill, and accepts
- **THEN** a pixel layer with that name, blend mode, and opacity is inserted
  above the selected layer filled with white

#### Scenario: The neutral color follows the mode

- **WHEN** the neutral fill is enabled with Screen selected
- **THEN** the created layer is filled black

#### Scenario: Clipping is not offered for a group

- **WHEN** the New Group dialog is opened
- **THEN** it shows no Use Previous Layer to Create Clipping Mask option and the
  created group is not clipped

#### Scenario: An unlisted mode defaults to transparent

- **WHEN** the neutral fill is enabled with a blend mode outside the documented
  lookup
- **THEN** the created layer is fully transparent

### Requirement: First-class Background layer

`pictura_core::Layer` SHALL carry an explicit background flag. The codec SHALL
derive the flag on read — the bottom top-level, non-group layer named
`Background` is the Background — and SHALL write the `Background` name for a
flagged layer on save. The `is_background` check SHALL use the flag and MUST NOT
depend on the layer's index or name. `Layer from Background…` SHALL clear the
flag and unlock the layer, and `Background From Layer` SHALL set the flag,
convert transparent pixels to the background color, move the node to the bottom
of the stack, and refuse a group or a layer that is already the Background. **A
Background converted to a normal layer SHALL be renamed to the next free
`Layer N` name** (through `next_layer_name(doc, "Layer")`) rather than keeping
the name `Background`; the conversion SHALL be reachable from the Layers panel
by a double-click on the Background row in its content band and by dropping the
Background on the New Layer button, and each of those panel gestures SHALL
present a **name-and-color dialog** (a name+color-only factory, or the unlock
followed by `set_layer_name_path` and `set_layers_color`) so the converted
layer's name and color label are chosen by the user, defaulting to `Layer N` and
no label. Each conversion SHALL recomposite and record exactly one undo state;
cancelling the dialog SHALL leave the Background unchanged and record nothing.

#### Scenario: The flag round-trips through PSD

- **WHEN** a document whose bottom layer is the Background is saved and read
  back
- **THEN** the re-read bottom layer reports the background flag

#### Scenario: The flag is independent of position and name

- **WHEN** a non-bottom layer named `Background` is present and a flagged layer
  sits at the bottom under another name
- **THEN** only the flagged layer is reported as the Background

#### Scenario: Convert from Background

- **WHEN** `Layer from Background…` runs on the Background layer
- **THEN** the flag is cleared, the layer is no longer locked, the layer is
  renamed to the next free `Layer N`, and the change is one undo step

#### Scenario: A double-click converts the Background [lmb_background_dblclick]

- **WHEN** the user double-clicks the Background row in its content band
- **THEN** a name-and-color dialog opens, and on accept the Background becomes a
  normal unlocked layer named by the user (default `Layer N`) with the chosen
  color label, in one undo step

#### Scenario: Dropping the Background on New Layer converts it [lmb_background_drop]

- **WHEN** the Background row is dropped on the New Layer strip button
- **THEN** the name-and-color dialog opens, and on accept the Background is
  converted in place to a normal `Layer N` layer with the chosen label, no
  `Background copy` is created, and the change is one undo step

#### Scenario: Cancelling the conversion dialog changes nothing [lmb_background_cancel]

- **WHEN** the conversion dialog is cancelled
- **THEN** the Background keeps its flag, lock, name, and pixels, and no history
  state is recorded

#### Scenario: Convert to Background

- **WHEN** `Background From Layer` runs on a normal pixel layer
- **THEN** the layer becomes the flagged Background at the bottom of the stack,
  its transparent pixels take the background color, and the change is one undo
  step

### Requirement: Layer via Copy and Layer via Cut

The system SHALL provide `Layer via Copy` (`Ctrl+J`) and `Layer via Cut`
(`Shift+Ctrl+J`). Each SHALL require an active selection; with no selection the
command SHALL be refused and the document unchanged. `Layer via Copy` SHALL
create a new pixel layer directly above the active layer containing only the
pixels inside the active selection. `Layer via Cut` SHALL do the same and SHALL
additionally clear those pixels from the source layer. Each SHALL preserve
every pixel outside the selection and SHALL recomposite and record exactly one
undo state.

#### Scenario: Layer via Copy extracts the selection

- **WHEN** a raster layer has an active selection and Layer via Copy runs
- **THEN** a new layer above it contains only the selected pixels and the source
  layer is unchanged

#### Scenario: Layer via Cut clears the source

- **WHEN** a raster layer has an active selection and Layer via Cut runs
- **THEN** a new layer above it contains the selected pixels and those pixels
  are cleared to transparency in the source layer

#### Scenario: No selection is refused

- **WHEN** Layer via Copy or Cut is invoked with no active selection
- **THEN** the command is unavailable, no layer is added, and the document is
  unchanged

### Requirement: Select Similar and Select Linked Layers

The system SHALL provide `Select Similar` and `Select Linked Layers`. `Select
Similar` SHALL select every layer that matches the active layer's kind and
attributes. `Select Linked Layers` SHALL select every member of the active
layer's link set. `Link Layers` SHALL add the selected layers to a link set,
joining existing sets when members already belong to one, and `Unlink Layers`
SHALL remove the selected layers from their sets and drop an emptied set. All
four SHALL be view/selection operations that do not change layer pixels and do
not add a history state. Link sets SHALL be transient session state: they SHALL
NOT be serialized, and a structural layer operation SHALL clear them (documented
limitation; the PSD link-set representation is unsourced).

#### Scenario: Select Similar matches the active layer

- **WHEN** Select Similar runs on a pixel layer
- **THEN** every layer of the same kind and matching attributes is selected and
  no pixels change

#### Scenario: Link and Select Linked

- **WHEN** two layers are linked and Select Linked Layers then runs on one of
  them
- **THEN** both members of the set are selected

#### Scenario: Unlink removes membership

- **WHEN** Unlink Layers runs on a linked layer
- **THEN** that layer is no longer in the set and linking/selecting no longer
  includes it

#### Scenario: Link sets are session-only

- **WHEN** a document with linked layers is saved and reopened, or a structural
  layer operation runs
- **THEN** the link sets are not persisted and are cleared by the structural
  operation

### Requirement: Delete Hidden Layers and Hide Layers

The system SHALL provide `Delete Hidden Layers`, which SHALL remove every layer
whose effective visibility is off, and `Hide Layers`, which SHALL set the
selected layers hidden. Each SHALL recomposite and record exactly one undo
state; `Delete Hidden Layers` on a document with no hidden layers SHALL record
no history state.

#### Scenario: Delete Hidden Layers removes hidden layers

- **WHEN** a document with hidden and visible layers runs Delete Hidden Layers
- **THEN** every hidden layer is removed, the visible layers remain, and the
  removal is one undo step

#### Scenario: Hide Layers hides the selection

- **WHEN** Hide Layers runs on one or more selected layers
- **THEN** each selected layer becomes hidden and the change is one undo step

### Requirement: Rasterize subset

The system SHALL provide a `Rasterize` submenu. `Fill Content` SHALL be
implemented: for a fill-content layer (a layer whose opaque adjustment block is
a fill-content key — `SoCo`, `GdFl`, or `PtFl` — and whose payload decodes to
`Adjustment::SolidFill`, `Adjustment::GradientFill`, or
`Adjustment::PatternFill`) it SHALL render the fill content to a full-layer
pixel node and clear the fill/adjustment data. A solid fill SHALL bake its
decoded colour across the layer rect; a gradient fill SHALL bake the generated
five-kind ramp with opaque alpha across the layer rect; a pattern fill SHALL
bake the pattern the document's `Patt` resource tiles over the layer rect (with
the pattern's own alpha, or the grey placeholder when the referenced pattern is
absent or was skipped as non-8-bit/malformed). `Shape` SHALL be implemented: a
shape layer (a solid fill cut to a `vmsk` vector mask) SHALL have its rendered
appearance — the fill cut to the outline, plus its stroke — baked into ordinary
pixels, and its shape/vector definition dropped (the solid-fill adjustment, the
`vmsk` vector mask, the live-shape `vogk` block, and the stroke effect), while
keeping its name, opacity, fill, blend mode, and layer mask. Its rect SHALL grow
when an effect (an outside or centered stroke) paints past it, so the baked
pixels cover the whole rendered appearance. `Rasterize
Layer` SHALL rasterize the active layer only when it is a fill-content layer,
and SHALL otherwise refuse without changing the document. `Rasterize All
Layers` SHALL rasterize every fill-content layer and every shape layer in the
document. The `Type`, `Vector Mask`, `Smart Object`, `Video`, and `3D` variants
SHALL remain visible and disabled, with a documented reason that those layer
kinds do not exist or are handled elsewhere. A rasterization whose content is
not decodable SHALL be disabled and SHALL refuse without changing the document,
and every applied rasterization SHALL recomposite and record exactly one undo
state.

#### Scenario: Fill Content becomes pixels

- **WHEN** Rasterize Fill Content runs on a solid fill-content layer
- **THEN** the layer becomes a pixel layer holding the rendered fill, its fill /
  adjustment data is cleared, and the change is one undo step

#### Scenario: A descriptor-form fill is rasterizable

- **WHEN** Rasterize Fill Content runs on a layer whose `SoCo` payload is the
  standard Photoshop descriptor
- **THEN** the layer becomes a pixel layer holding the descriptor's colour and
  the change is one undo step

#### Scenario: A gradient fill is rasterizable

- **WHEN** Rasterize Fill Content runs on a layer whose `GdFl` payload decodes to
  `Adjustment::GradientFill`
- **THEN** the layer becomes a pixel layer holding the generated gradient over
  its rect, its fill / adjustment data is cleared, and the change is one undo
  step

#### Scenario: A pattern fill is rasterizable

- **WHEN** Rasterize Fill Content runs on a layer whose `PtFl` payload decodes to
  `Adjustment::PatternFill`
- **THEN** the layer becomes a pixel layer holding the tiled pattern over its
  rect, its fill / adjustment data is cleared, and the change is one undo step

#### Scenario: A pattern fill with a missing pattern is still rasterizable

- **WHEN** Rasterize Fill Content runs on a `PtFl` layer whose `pattern_id` is
  not present in the document's `Patt` resource
- **THEN** the layer becomes a pixel layer holding the grey placeholder and
  the change is one undo step

#### Scenario: A shape layer rasterizes to pixels

- **WHEN** Rasterize Shape runs on a shape layer
- **THEN** the layer's fill cut to its outline (and its stroke) is baked into
  the layer's pixels, its fill/adjustment, vector mask, live-shape, and stroke
  data are dropped, its appearance is unchanged when recomposited, and the
  change is one undo step

#### Scenario: Rasterize Shape refuses a non-shape

- **WHEN** Rasterize Shape runs on a pixel, group, adjustment, type, or
  Background layer
- **THEN** the document is unchanged and no undo state is recorded

#### Scenario: A kind-less variant stays disabled

- **WHEN** the Rasterize submenu is shown
- **THEN** Type, Vector Mask, Smart Object, Video, and 3D are visible but
  disabled, and their reason is documented

#### Scenario: Rasterize Layer refuses a non-fill layer

- **WHEN** Rasterize Layer runs on a group, adjustment, or plain pixel layer
- **THEN** the document is unchanged and no undo state is recorded

### Requirement: Management commands recomposite and record one state

Every management command in this capability SHALL run through the bridge, SHALL
recompute the document composite before recording, and SHALL add exactly one
undo state per invocation. A command that changes no layer SHALL record no
history state and emit no document change.

#### Scenario: A no-op records no history

- **WHEN** a management command is invoked and its validation finds nothing to
  change
- **THEN** no history state is added and the document is unchanged

#### Scenario: One state per command

- **WHEN** a management command completes
- **THEN** exactly one undo state is recorded and one undo restores the prior
  tree and pixels

### Requirement: Selecting, copying, and moving on an invisible active layer

When the active layer is invisible, the application SHALL still allow
selection, copy, and Move operations on it, while paint and filter edits are
refused. The Move tool SHALL translate an invisible active layer exactly as a
visible one (mouse drag and keyboard nudge), and `compute_move_preview` SHALL
save and restore the active layer's `visible` flag around building its preview
base so a move never makes an invisible layer visible as a side effect. A move
on an invisible layer SHALL still record exactly one history state and SHALL
leave the layer invisible after commit.

#### Scenario: Move works on an invisible layer [lmg_move_invisible]

- **WHEN** the active layer is invisible and the Move tool drags it by a delta
- **THEN** the layer's content is translated, the layer stays invisible after
  commit, and one history state is recorded

#### Scenario: The preview does not reveal the layer [lmg_move_invisible_preview]

- **WHEN** the Move drag begins on an invisible active layer
- **THEN** `compute_move_preview` preserves the prior `visible` value and the
  document's layer visibility is unchanged by starting the drag

#### Scenario: Selection and copy stay available [lmg_select_invisible]

- **WHEN** the active layer is invisible and a selection tool or a copy command
  runs
- **THEN** the selection and copy operate on the layer's content and are not
  refused

### Requirement: Arrange layers within the container

The system SHALL provide the `Layer > Arrange` commands **Bring to Front**,
**Bring Forward**, **Send Backward**, and **Send to Back**. Each SHALL move the
active layer within its own container only — never into a sibling container or
a group — where the top of the stack is the front. Bring to Front SHALL move
the layer to the top of its container, Bring Forward SHALL move it one position
towards the front, Send Backward SHALL move it one position towards the back,
and Send to Back SHALL move it to the bottom-most position that is above any
Background in the container. A move that would leave the layer in its current
position SHALL be refused and record no history state, and the command SHALL be
disabled when no move is possible. The Background layer and a fully-locked
layer MUST NOT reorder. Every applied arrange SHALL recomposite and record
exactly one undo state.

#### Scenario: Bring to Front moves the active layer to the top

- **WHEN** a layer is selected in a container with other layers above it and
  Bring to Front runs
- **THEN** the layer is the topmost node of its container, the relative order of
  the other nodes is unchanged, and the change is one undo step

#### Scenario: Send to Back moves the active layer to the bottom

- **WHEN** a layer is selected in a container without a Background and Send to
  Back runs
- **THEN** the layer is the bottom-most node of its container and the change is
  one undo step

#### Scenario: Bring Forward and Send Backward move one position

- **WHEN** Bring Forward runs on a layer with a node above it
- **THEN** the layer and that node swap positions, and Send Backward performs
  the inverse swap

#### Scenario: A boundary move is refused and disabled

- **WHEN** the active layer is already at the front and Bring Forward or Bring
  to Front is invoked
- **THEN** the command is disabled, the document is unchanged, and no history
  state is recorded

#### Scenario: The Background does not reorder

- **WHEN** the Background layer is the active layer and any Arrange command runs
- **THEN** the command is disabled and the document is unchanged

#### Scenario: Send to Back stops above the Background

- **WHEN** a container has a Background and Send to Back or Send Backward runs
  on the layer directly above it
- **THEN** Send to Back places the layer directly above the Background, Send
  Backward is refused as a boundary no-op, and the Background stays bottom-most

### Requirement: Reverse selected layers

The system SHALL provide `Layer > Arrange > Reverse`, which SHALL reverse the
stacking order of the selected layers. The selection MUST contain at least two
layers that share one container and occupy a contiguous run of positions in
that container; otherwise the command SHALL be refused, the document SHALL be
unchanged, and no history state SHALL be recorded. The command MUST refuse when
any selected layer is the Background or is fully locked. A successful reverse
SHALL recomposite and record exactly one undo state.

#### Scenario: A contiguous run reverses in place

- **WHEN** three adjacent layers in one container are selected and Reverse runs
- **THEN** their order within the run is reversed, the nodes before and after
  the run keep their positions, and the change is one undo step

#### Scenario: A non-contiguous selection is refused

- **WHEN** the selected layers share a container but are not contiguous
- **THEN** the command is refused, the document is unchanged, and no history
  state is recorded

#### Scenario: A cross-container selection is refused

- **WHEN** the selected layers span more than one container and Reverse runs
- **THEN** the command is refused, the document is unchanged, and no history
  state is recorded

### Requirement: Delete Layer

The system SHALL provide a `Layer > Delete Layer` command that deletes every
selected layer. The Background and fully-locked layers MUST NOT be deleted. A
selected node whose ancestor is also selected SHALL be deleted once, with its
ancestor. When at least one layer is deleted the command SHALL recomposite and
record exactly one undo state; when nothing is deletable it SHALL record no
history state. The command SHALL be disabled when no document is open or no
layer is selected.

#### Scenario: Delete removes the selected layers

- **WHEN** one or more ordinary layers are selected and Delete Layer runs
- **THEN** those layers are removed and the removal is one undo step

#### Scenario: The Background is not deletable

- **WHEN** the only selected layer is the Background and Delete Layer runs
- **THEN** the command is disabled, the document is unchanged, and no history
  state is recorded

### Requirement: Merge Down as a distinct command

The system SHALL provide a distinct `Layer > Merge Down` command that merges
the active layer with the layer directly below it within the same container,
reusing the document compositor and the Merge Down rules (raster target, a
layer must exist below, one undo state). This leaf SHALL be separate from the
`Layer > Merge Layers` command; `Ctrl+E` SHALL remain bound to Merge Layers.
Merge Down SHALL be disabled when the active layer has no layer directly below
it or either layer is an adjustment or fill-content layer.

#### Scenario: Merge Down is reachable as its own leaf

- **WHEN** the `Layer` menu is opened with a pixel layer sitting directly above
  another pixel layer
- **THEN** `Merge Down` is present, enabled, and distinct from `Merge Layers`

#### Scenario: Merge Down replaces the pair in place

- **WHEN** Merge Down runs on a pixel layer with a raster layer directly below
- **THEN** the two layers are replaced by one pixel layer at the lower layer's
  position that inherits the lower layer's name, blend mode, and opacity, and
  the change is one undo step

#### Scenario: Merge Down is disabled without a layer below

- **WHEN** the bottom-most layer of the stack is the active layer
- **THEN** Merge Down is disabled and the document is unchanged

### Requirement: Stamp Visible and Stamp Selected

The system SHALL provide `Layer > Stamp Visible` and `Layer > Stamp Selected`,
which composite the currently eye-visible (respecting ancestor visibility) or
selected layers respectively into a NEW raster pixel layer inserted directly
above the active layer. The source layers MUST be left intact — the command is
non-destructive and MUST NOT remove, merge, or hide any existing layer. Each
command SHALL refuse, without changing the document and without recording
history, when the active layer does not resolve, when no input layer is
eligible, or (for Stamp Visible) when the active layer is hidden. A successful
stamp SHALL recomposite and record exactly one undo state.

#### Scenario: Stamp Visible adds one layer and keeps the originals

- **WHEN** a document has several visible layers and Stamp Visible runs with an
  active layer
- **THEN** exactly one new raster layer is inserted directly above the active
  layer whose pixels equal the composite of the visible layers, and every
  original layer still exists and is unchanged

#### Scenario: A hidden layer is excluded and survives

- **WHEN** a hidden layer is present and Stamp Visible runs
- **THEN** the hidden layer is not part of the composite and remains in place
  unchanged

#### Scenario: Stamp Selected composites only the selection

- **WHEN** two layers are selected and Stamp Selected runs
- **THEN** one new raster layer above the active layer holds the composite of
  just those selected layers and both originals remain

#### Scenario: A hidden active layer is refused for Stamp Visible

- **WHEN** the active layer is hidden and Stamp Visible is invoked
- **THEN** the command is disabled, the document is unchanged, and no history
  state is recorded

#### Scenario: A stamp is one undo step

- **WHEN** the user undoes a stamp
- **THEN** the added layer is removed and the prior layer tree and pixels are
  restored

### Requirement: Copy and Paste Shape Attributes

The system SHALL provide `Copy Shape Attributes` and `Paste Shape Attributes`
for shape layers. Copy SHALL capture a shape layer's fill and stroke
appearance: whether the fill is on and its colour, and whether the stroke is on
with its colour, width, and alignment. Paste SHALL apply the captured
appearance to a shape layer, replacing its fill and stroke; a target that is not
a shape layer MUST be refused, leaving it unchanged and adding no history state.
Copy SHALL NOT change the document and SHALL add no history state. Paste that
changes the layer SHALL recomposite and record exactly one undo state; a paste
that changes nothing SHALL add no history state. Both SHALL be reachable from
the `Layer` menu and from the Layers row context menu for a shape layer.

#### Scenario: Copy then paste transfers fill and stroke

- **WHEN** Copy Shape Attributes runs on a shape layer with a fill and a
  stroke, and Paste Shape Attributes then runs on a different shape layer
- **THEN** the target's fill colour and stroke colour, width, and alignment
  equal the source's, and the paste is one undo step

#### Scenario: Pasting onto a non-shape is refused

- **WHEN** Paste Shape Attributes runs with a pixel, group, adjustment, type, or
  Background layer as the target
- **THEN** the target is unchanged and no history state is recorded

#### Scenario: Copying a shape changes nothing

- **WHEN** Copy Shape Attributes runs on a shape layer
- **THEN** the document is unchanged and no history state is recorded

#### Scenario: A paste with no change adds no history

- **WHEN** Paste Shape Attributes applies the same fill and stroke the target
  already has
- **THEN** the layer is unchanged and no history state is recorded

### Requirement: The Background remains bottom-most

Within every container, the node carrying the Background flag SHALL remain the
bottom-most node. A container SHALL contain at most one Background, and a
document SHALL contain at most one Background overall. No other node SHALL be
inserted at a child index at or below the Background's index. A within-container
delta move SHALL clamp at the position directly above the Background; a delta
move whose source is already below the Background SHALL be refused rather than
jump a sibling. A move-to-target and its dry run SHALL both refuse a destination
at or below the Background. A container nested inside a group SHALL follow the
same rule when it contains a Background. The rule SHALL use the Background flag
and MUST NOT depend on the node's name or index.

#### Scenario: A move below the Background is refused

- **WHEN** a normal layer is moved to the position below a Background in the
  same container
- **THEN** the move is refused, the dry run reports the same refusal, and the
  document is unchanged

#### Scenario: A delta move clamps at the Background

- **WHEN** a layer directly above a Background is moved towards the back by any
  negative delta
- **THEN** the layer stays directly above the Background and records no change

#### Scenario: A delta move from below the Background is refused

- **WHEN** a document is in the invalid state where a layer sits below a flagged
  Background and that layer is moved by a delta
- **THEN** the move is refused so no sibling is jumped, and the document is
  unchanged

#### Scenario: A second Background is refused

- **WHEN** `Background From Layer` runs on a normal layer while the document
  already contains a Background
- **THEN** the conversion is refused and the document is unchanged

#### Scenario: A nested group applies the same floor

- **WHEN** a group contains a Background and a layer above it is moved below
  that Background
- **THEN** the move is refused and the nested Background stays bottom-most
