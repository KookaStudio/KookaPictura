# canvas-mask-domain Specification

## Purpose

Makes raster masks editable throughout the canvas while retaining coverage
beyond it, with black movement backgrounds and lossless off-canvas storage.

## Requirements

### Requirement: Canvas-wide edits preserve existing mask coverage

Every raster mask SHALL be editable at every in-canvas coordinate, including
coordinates outside its owner's content and its stored rectangle. This SHALL
apply to newly created masks and existing masks loaded from PSD/PSB. Expanding
an imported mask for editing SHALL preserve its stored samples, unrelated
metadata, and pre-edit in-canvas coverage. For decoded masks, that coverage
SHALL use explicit stored samples and the existing exterior default outside
the stored rectangle. For a data-less imported mask, canvas coverage SHALL
start at 255 regardless of its stored default, preserving the existing
missing-data rendering and solo/Channels semantics. Canvas edits SHALL affect
only their requested canvas region and SHALL
preserve stored off-canvas samples. Merely viewing a mask or selecting its
thumbnail SHALL NOT expand stored bounds, dirty the document, or add history.

#### Scenario: Paint outside a small layer and imported mask [cmd_canvas_paint]

- **WHEN** Brush, Pencil, and Eraser each edit a point on the canvas outside a small layer and its smaller imported mask
- **THEN** the requested mask coverage changes, the layer's pixels remain untouched, and the other mask coverage retains its prior values

#### Scenario: Fill tools reach the canvas margins [cmd_canvas_fill]

- **WHEN** Edit Fill, Gradient, Paint Bucket, Edit Stroke, Fill Path, Stroke Path, and shape Fill Pixels each target a selected canvas region beyond the layer and imported mask bounds
- **THEN** each edits mask coverage in that region at the correct coordinates without changing layer pixels or retained off-canvas samples

#### Scenario: Filters reach the canvas margins [cmd_canvas_filter]

- **WHEN** an adjustment or destructive filter edits an imported mask whose stored rectangle is smaller than the canvas
- **THEN** the operation can change coverage beyond the stored rectangle while coverage outside the requested canvas region stays unchanged

#### Scenario: Import expansion preserves white or gray exterior coverage [cmd_import_defaults]

- **WHEN** a smaller imported mask with decoded coverage and a white or gray exterior default receives its first canvas edit
- **THEN** unedited canvas pixels outside its original stored rectangle keep that prior coverage and its existing stored pixels and refinement settings are preserved

#### Scenario: A data-less import keeps its visible coverage on first edit [cmd_data_less_import]

- **WHEN** a data-less imported mask with a nonwhite stored default receives a
  local canvas edit
- **THEN** its pre-edit composite and solo coverage are 255, unedited canvas
  coverage remains 255 after the edit, and undo restores its original
  missing-data state, stored bounds, default, and unrelated metadata

#### Scenario: Viewing an imported mask changes no document state [cmd_read_only]

- **WHEN** an imported mask, including one without decoded data, is selected, shown solo, and displayed in Layers and Channels thumbnails without editing it
- **THEN** its stored rectangle and samples, document modified state, and history remain unchanged

### Requirement: Whole-mask moves expose black without discarding coverage

A whole-mask integer translation SHALL expose zero coverage wherever the moved
mask has no stored samples, independently of the color swatches. It SHALL retain
every stored nonzero sample beyond every canvas edge, including negative
coordinates, and SHALL NOT clip, threshold, quantize, or discard such samples
because they are off-canvas. A translation and its inverse SHALL restore all
coverage values exactly. Pure translations SHALL also preserve any retained
native 16-bit or 32-bit mask samples without reducing their precision.
An unlinked mask selected as the edit target SHALL move independently of its
owner; a mask carried by an existing linked-layer move SHALL obey the same
black-exterior and retention rules. A subsequent canvas edit SHALL NOT erase
the retained off-canvas data.

#### Scenario: Whole-mask moves expose black with white swatches [cmd_move_black]

- **WHEN** an imported mask with a white exterior default is moved after setting both color swatches to white
- **THEN** the newly uncovered canvas area is black and existing moved coverage keeps its values

#### Scenario: Coverage returns from every canvas edge [cmd_move_return]

- **WHEN** a mask containing values 0, 1, 64, 128, 254, and 255 is moved partially and then completely outside each canvas edge and corner, and moved back by the inverse offsets
- **THEN** every original coverage sample is restored byte-for-byte, including the partially covered pixels

#### Scenario: A canvas edit preserves off-canvas coverage [cmd_offcanvas_edit]

- **WHEN** a mask is moved off-canvas, an in-canvas region is painted, filled, or filtered, and the retained mask coverage is moved back
- **THEN** the off-canvas samples that were outside the edit region return with exactly their previous values

#### Scenario: Native-depth nonzero values survive moves [cmd_native_move]

- **WHEN** 16-bit and 32-bit imported masks containing positive samples too small to survive conversion to 8-bit are moved off-canvas and back
- **THEN** the retained native samples return exactly, with 32-bit values retaining their original bit patterns

#### Scenario: Mask targeting respects the existing link state [cmd_move_target]

- **WHEN** the Move tool drags an unlinked mask edit target, then drags a layer carrying a linked mask
- **THEN** the first gesture moves only mask coverage and the second carries the linked mask with its owner, with black exterior coverage in both cases

### Requirement: Raster mask thumbnails show opaque document-space coverage

Raster mask thumbnails SHALL sample the mask's stored grayscale coverage over
the document rectangle, preserve the document aspect ratio, and have opaque
pixels throughout the image. Zero-coverage margins SHALL be black, not
transparent or checkerboard. This rule SHALL apply independently of the owning
layer's bounds and the layer-color thumbnail's contents option. Disabled masks
SHALL still display their stored coverage. Thumbnail generation SHALL sample
only its output grid and SHALL NOT allocate or scan a full-document mask image.
For a data-less imported mask, the thumbnail SHALL sample coverage 255 without
materializing or changing its stored missing-data state or default color.

#### Scenario: A small mask has opaque black thumbnail margins [cmd_thumb_black]

- **WHEN** a new mask on a small square layer in a wide document is displayed with Entire Document and Layer Bounds thumbnail options
- **THEN** both raster mask thumbnails have the document aspect, white content at its document position, and opaque black zero-coverage margins

#### Scenario: Thumbnail reads stay bounded and ignore disabled state [cmd_thumb_bounded]

- **WHEN** a large imported mask is displayed as a small thumbnail while enabled and disabled
- **THEN** both thumbnails show the same stored grayscale coverage and sample only the thumbnail grid without materializing a document-sized intermediate

### Requirement: Mask allocation and gestures are atomic

Mask creation, domain expansion, and movement requiring additional storage SHALL
reserve the required allocations through the existing memory budget before
publishing any mutation. An allocation refusal or unrepresentable rectangle
SHALL leave mask samples, bounds, native data, layer pixels, selection, active
target, modified state, and history unchanged, and SHALL report failure.
Successful first edits SHALL include domain expansion in their one normal undo
state. Cancelled gestures and previews SHALL restore the exact prior state;
zero-offset moves SHALL add no state and SHALL NOT normalize an imported mask.

#### Scenario: Allocation or coordinate failure leaves no mutation [cmd_refusal]

- **WHEN** mask creation, first canvas edit, or a retained-coverage move exceeds the memory budget or representable coordinate range
- **THEN** the operation reports failure and the document, mask, selection, active target, and history equal their pre-operation state

#### Scenario: The first expanded edit undoes in one step [cmd_undo]

- **WHEN** the first edit of an imported mask expands its stored domain and commits, then undo and redo run
- **THEN** exactly one normal operation state is recorded, undo restores the original bounds and samples, and redo restores the expanded edited mask including off-canvas data

#### Scenario: Cancel and zero movement preserve imported storage [cmd_cancel]

- **WHEN** a mask stroke, filter preview, or move on a decoded or data-less import is cancelled, or a move ends at zero offset
- **THEN** the original mask samples, metadata, stored bounds, and history are unchanged

### Requirement: Saved masks retain off-canvas coverage

PSD and PSB save/reopen SHALL preserve retained off-canvas coverage, signed mask
bounds, black movement exterior, and unrelated mask metadata, independently of
the canvas dimensions. Opening an existing file without editing its mask SHALL
NOT normalize or recolor the imported mask. A saved mask moved back after
reopening SHALL recover its retained coverage exactly at the saved sample depth.

#### Scenario: Off-canvas samples survive save and reopen [cmd_saved_return]

- **WHEN** a mask with fractional coverage is moved completely off-canvas, saved to PSD and PSB, reopened, and moved back
- **THEN** all retained coverage values return exactly, the exterior stays black, and unrelated mask metadata survives
