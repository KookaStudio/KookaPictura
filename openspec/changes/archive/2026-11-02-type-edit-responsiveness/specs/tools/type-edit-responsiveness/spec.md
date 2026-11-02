## ADDED Requirements

### Requirement: Type layer edits refresh only the text they touch

Re-setting a type layer's font, size, alignment, colour, or paragraph options, and
committing a type edit session, SHALL refresh only the union of the layer's reach
before and after the edit, and SHALL record one history state. The refreshed document
SHALL match a whole-document composite of the same layers. A raster nearly the size of
the canvas SHALL count only its inked pixels when nothing else extends its reach. A
knockout elsewhere in the document SHALL NOT widen the refresh. An edit whose reach
cannot be proven, or whose damage covers three quarters of the canvas or more, SHALL
refresh the whole document.

#### Scenario: A style change refreshes the text [ter_commit]

- **WHEN** the size, font family, colour, or a paragraph option of a small type layer
  in a large document changes
- **THEN** the damaged region lies inside the union of the old and new text bounds,
  and one history state is recorded

#### Scenario: A change of alignment clears the old place [ter_moved]

- **WHEN** a type layer's alignment changes so its raster moves
- **THEN** the old and new places are both refreshed, and the document matches a fresh
  document holding the same type layer

#### Scenario: Undo restores the pixels [ter_undo]

- **WHEN** an edit that resizes or moves the raster is undone and then redone
- **THEN** each step shows exactly the pixels it showed before

#### Scenario: A knockout stays bounded and exact [ter_knockout]

- **WHEN** a layer under a knockout changes its pixels in place
- **THEN** the reach is the layer's own, and that region of a fresh composite equals
  the same region of a whole-document composite

#### Scenario: A huge box refreshes its ink [ter_box]

- **WHEN** a few words in a box that nearly fills the canvas change colour
- **THEN** the damaged region is the words, not the box

#### Scenario: An effect keeps the raster's reach [ter_box_effect]

- **WHEN** a type layer in such a box has an effect
- **THEN** the reach is not trimmed

### Requirement: Undo across a history segment stays bounded

Undoing or redoing an edit that started a new history segment, such as one that resized
a type raster, SHALL refresh only the composite pixels that differ between the two
states. It SHALL refresh the whole document when the canvas geometry, a mask, or a
document channel differs, and when no composite pixel differs.

#### Scenario: Undoing a resize refreshes the text [ter_undo_region]

- **WHEN** a change of size that resized the raster is undone and redone
- **THEN** each step damages a region around the text, and the pixels are restored

#### Scenario: The history reports the changed tiles [ter_crossing_region]

- **WHEN** a walk crosses a segment and the composite differs in one tile
- **THEN** the restore reports that tile's region

#### Scenario: A channel change refreshes everything [ter_crossing_full]

- **WHEN** a walk crosses a segment and a document channel differs
- **THEN** the restore reports the whole document

### Requirement: Type previews and sessions stay text-sized

A live type preview, a Warp Text preview and its cancel, a type style dialog preview,
and hiding or showing a layer for a type edit session SHALL refresh only the reach of
the layers they change and SHALL record no history. When a pending preview, a soloed
mask, or a live session owns the display, they SHALL refresh the whole document. A
commit that follows previews SHALL record every change since the last history state,
including the layer's place before the first preview.

#### Scenario: A preview is bounded and unrecorded [ter_preview]

- **WHEN** a type layer is previewed with another colour
- **THEN** the damaged region lies inside the text bounds, one change is notified, and
  the history does not move

#### Scenario: Opening and cancelling restore the document [ter_session]

- **WHEN** a type edit session opens and is cancelled
- **THEN** each step damages only the text bounds, and the document shows exactly the
  pixels it showed before

#### Scenario: A committed session matches a fresh document [ter_session_commit]

- **WHEN** a type edit session changes the text and commits
- **THEN** the document matches a fresh document holding the same type layer

#### Scenario: A soloed mask keeps the full path [ter_display_owner]

- **WHEN** a type layer is previewed while a mask is soloed
- **THEN** the whole document refreshes

#### Scenario: A warp preview is bounded and cancel restores [ter_warp_preview]

- **WHEN** a type layer is previewed with a warp and the dialog is cancelled
- **THEN** the preview damages a region, records nothing, and cancelling restores the
  pixels

#### Scenario: Undoing a warp restores what the previews passed [ter_warp_commit]

- **WHEN** a strong warp is previewed, a slight one is committed, and it is undone
- **THEN** the document shows exactly the pixels from before the dialog opened

#### Scenario: A style preview is bounded [ter_style_preview]

- **WHEN** a paragraph style applied to a type layer is previewed with another colour
- **THEN** the damaged region lies inside the text, and the history does not move

### Requirement: The tool profiler times type edits

The control server SHALL offer `type_op` to add a type layer, re-set or preview it, and
preview or cancel a warp. `scripts/profile-tools.py` SHALL time one scenario for each
of font, size, alignment, colour, paragraph options, colour preview, warp preview, and
a canvas-sized box, and SHALL report the cost of undoing an edit.

#### Scenario: The control server re-sets a type layer [ter_type_op]

- **WHEN** a client adds a type layer through `type_op`, re-sets it, previews it,
  previews a warp, and cancels the warp
- **THEN** each call succeeds, and an unknown operation is refused
