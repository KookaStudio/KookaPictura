# document/auto-blend-layers Specification

## Purpose
Edit > Auto-Blend Layers: when it applies, the masks it generates for a panorama
or an image stack, its clipped tonal correction, and how a run is cancelled and
undone. The algorithm is an approximation with behavioural parity only.

## Requirements

### Requirement: Auto-Blend availability

Edit > Auto-Blend Layers SHALL be available only for an RGB or Grayscale document
with at least two eligible selected layers and no pending modal transform. It
MUST be disabled when a selected layer is the Background, a Smart Object or a
clipped layer; a Smart Object outside the selection MUST NOT disable it. Groups,
adjustment and fill layers, type and shape layers, hidden layers and empty layers
in the selection SHALL be skipped without a mask and SHALL NOT count as eligible.
Locked layers SHALL be processed. The skip and clipped-layer rules are inferred.

#### Scenario: Refused selections and modes [abl_refused]

- **WHEN** the `auto_blend` tests ask for availability in Lab, CMYK, Indexed and Bitmap documents, and with a Background, Smart Object or clipped layer selected, or one eligible layer
- **THEN** each is unavailable and running the command leaves the document unchanged

#### Scenario: A Smart Object outside the selection [abl_outside_selection]

- **WHEN** two raster layers are selected in a document that also holds a Smart Object
- **THEN** the command is available

#### Scenario: Skipped and locked layers [abl_skipped]

- **WHEN** the selection holds two raster layers, one locked, plus a group, an adjustment, a hidden layer, a type layer and an empty layer
- **THEN** only the two raster layers gain masks

#### Scenario: The menu follows availability [abl_menu]

- **WHEN** the Edit menu is shown with one and with two raster layers selected
- **THEN** Auto-Blend Layers is disabled and then enabled

### Requirement: Auto-Blend dialog and history

The command SHALL open a dialog offering Panorama and Stack Images, defaulting to
Panorama, and a Seamless Tones And Colors checkbox defaulting to off. A completed
run SHALL add exactly one history state; one undo SHALL restore every mask and
remove every correction. It SHALL run with a progress indicator and Cancel;
cancelling SHALL leave the document unchanged with no history state. The processed
layers SHALL stay selected after the commit.

#### Scenario: Dialog defaults [abl_dialog]

- **WHEN** the dialog opens
- **THEN** Panorama is chosen and Seamless Tones And Colors is off

#### Scenario: One state and one undo [abl_undo]

- **WHEN** Auto-Blend runs on two layers with Seamless on, then undo runs
- **THEN** history grew by one state labelled Auto-Blend Layers, and undo restores the composite and the layer count

#### Scenario: Cancelling [abl_cancel]

- **WHEN** a run is cancelled part way
- **THEN** the document and the history are unchanged

#### Scenario: A document edited during the run [abl_stale]

- **WHEN** the document changes while the worker runs
- **THEN** the plan is dropped, an error is reported and no state is added

#### Scenario: Selection after commit [abl_selection]

- **WHEN** a run inserts correction layers
- **THEN** the processed layers are still the selected layers

### Requirement: Generated masks

Auto-Blend SHALL create or replace one raster layer mask per processed layer and
MUST NOT change layer pixels. Analysis SHALL run at the document's depth and the
result SHALL stay at that depth. Existing raster masks SHALL be ignored, so the
same input gives byte-identical output.

#### Scenario: Stack Images keeps the sharpest detail [abl_focus]

- **WHEN** two aligned RGB layers, each sharp where the other is blurred, are blended with Stack Images and Seamless on
- **THEN** each layer gains a mask and the composite's focus metric beats each input over the overlap

#### Scenario: Stack Images prefers good exposure [abl_exposure]

- **WHEN** an exposure bracket of two layers is blended
- **THEN** blown and crushed regions come from the better-exposed layer

#### Scenario: Panorama seam avoids a difference [abl_seam]

- **WHEN** two overlapping layers differ only around an object inside the overlap
- **THEN** the object comes wholly from one layer

#### Scenario: Panorama keeps unique content [abl_no_overlap]

- **WHEN** two layers do not overlap, or one layer lies inside another
- **THEN** non-overlapping layers keep all content, and a contained layer's mask is black where the other layer wins

#### Scenario: Layers beyond the canvas and in groups [abl_geometry]

- **WHEN** a layer extends off the canvas, and layers sit in different groups
- **THEN** each mask matches its layer rect and blending uses document coordinates

#### Scenario: Depth is preserved [abl_depth]

- **WHEN** 16-bit and 32-bit RGB layers are blended
- **THEN** the document depth and layer planes are unchanged and each layer gains a mask

#### Scenario: A replaced mask gates the composite [abl_replace_mask]

- **WHEN** a mask is replaced at the same rect on a 16-bit layer holding a retained native mask, then undone, and separately saved and reloaded
- **THEN** the new mask gates the composite, undo restores the old one, and the reload shows the new mask

#### Scenario: Determinism and the work limit [abl_determinism]

- **WHEN** the same input runs twice, and a document above the work limit runs
- **THEN** masks are byte-identical, and the oversized run stays within the work-grid limit

### Requirement: Seamless Tones And Colors

With Seamless on, Auto-Blend SHALL add one clipped Curves layer named
`Auto-Blend Tones` directly above each layer needing a correction, below any
existing clipped run, and SHALL add none where the correction is identity. A later
run SHALL replace such a layer whether or not Seamless is on; a renamed layer
SHALL be left alone. Layer pixels MUST NOT change.

#### Scenario: Exposure mismatch is corrected [abl_seamless]

- **WHEN** two overlapping layers differ by a global gain with Seamless on, in RGB and in Grayscale
- **THEN** a clipped Curves layer narrows the difference over the overlap and pixel planes are byte-identical

#### Scenario: Identity adds nothing [abl_identity]

- **WHEN** the layers already match
- **THEN** no correction layer is inserted

#### Scenario: Placement below a clipped run [abl_placement]

- **WHEN** a processed layer already has a clipped layer above it, in a nested group
- **THEN** the correction sits directly above the base and paths remap correctly

#### Scenario: A re-run replaces the correction [abl_rerun]

- **WHEN** Auto-Blend runs twice with Seamless on, then once with it off, and a correction was renamed before a run
- **THEN** corrections do not stack, the off run removes them, and the renamed layer stays
