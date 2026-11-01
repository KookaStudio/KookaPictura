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
Panorama, each with an illustration, a Seamless Tones And Colors checkbox
defaulting to off, and a Content Aware Fill Transparent Areas checkbox defaulting
to off and disabled at 16 and 32 bits. A completed
run SHALL add exactly one history state; one undo SHALL restore every mask and
remove the merged layer. It SHALL run with a progress indicator and Cancel;
cancelling SHALL leave the document unchanged with no history state. The processed
layers SHALL stay selected after the commit.

#### Scenario: Dialog defaults [abl_dialog]

- **WHEN** the dialog opens
- **THEN** Panorama is chosen, both checkboxes are off, and each method shows an illustration

#### Scenario: Content Aware Fill follows depth [abl_fill_depth]

- **WHEN** the dialog opens for an 8-bit and for a 16-bit document
- **THEN** the option is enabled and then disabled

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

- **WHEN** a run adds the merged layer
- **THEN** the processed layers are still the selected layers

### Requirement: Generated masks

Auto-Blend SHALL create or replace one raster layer mask per processed layer and
MUST NOT change those layers' pixels. Every run SHALL also add one raster layer
named `<top layer> (merged)` above the processed layers and any clipped run on
the topmost, holding a multiband blend of the processed layers. Analysis SHALL run
at the document's depth and the result SHALL stay at that depth. Existing raster
masks SHALL be ignored, so the same input gives byte-identical output.

#### Scenario: Stack Images keeps the sharpest detail [abl_focus]

- **WHEN** two aligned RGB layers, each sharp where the other is blurred, are blended with Stack Images and Seamless on
- **THEN** each layer gains a mask and the composite's focus metric beats each input over the overlap

#### Scenario: Stack Images blends softly [abl_soft]

- **WHEN** two flat layers of different tones are stacked with detail on opposite sides
- **THEN** the merged layer has no hard step between the regions

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
- **THEN** the document depth and source planes are unchanged, each layer gains a mask, and the merged layer retains native samples

#### Scenario: A replaced mask gates the composite [abl_replace_mask]

- **WHEN** a mask is replaced at the same rect on a 16-bit layer holding a retained native mask, then undone, and separately saved and reloaded
- **THEN** the new mask gates the composite, undo restores the old one, and the reload shows the new mask

#### Scenario: Determinism and the work limit [abl_determinism]

- **WHEN** the same input runs twice, and a document above the work limit runs
- **THEN** masks are byte-identical, and the oversized run stays within the work-grid limit

#### Scenario: Placement above a clipped run [abl_placement]

- **WHEN** the topmost processed layer has a clipped layer above it, in a nested group
- **THEN** the merged layer sits above that run in the same group and no source path moves

#### Scenario: A re-run replaces the merged layer [abl_rerun]

- **WHEN** Auto-Blend runs twice, and a merged layer was renamed before a run
- **THEN** merged layers do not stack, and the renamed layer stays

### Requirement: Seamless corrections in the merged layer

With Seamless on, Auto-Blend SHALL bake per-channel gain corrections into the
merged layer. Source layer pixels MUST NOT change and no adjustment layer SHALL be
added.

#### Scenario: Exposure mismatch is corrected [abl_seamless]

- **WHEN** two overlapping layers differ by a global gain with Seamless on, in RGB and in Grayscale
- **THEN** the merged layer narrows the difference, no adjustment layer exists, and source pixels are byte-identical

#### Scenario: Matching layers are unchanged [abl_identity]

- **WHEN** the layers already match
- **THEN** the merged layer keeps their tone

### Requirement: Content Aware Fill Transparent Areas

With this post-CS6, non-parity option on, an 8-bit run SHALL synthesize the merged
layer's transparent canvas pixels from its content, so it covers the canvas. At
16 and 32 bits the option SHALL have no effect.

#### Scenario: Gaps are filled [abl_fill]

- **WHEN** two layers leave part of the canvas uncovered and the option is on
- **THEN** the merged layer is opaque across the canvas with colours from its content

#### Scenario: High depth ignores the option [abl_fill_ignored]

- **WHEN** a 16-bit run has the option on
- **THEN** the merged layer keeps its transparency

### Requirement: Stack Images selection and tone

Stack Images SHALL leave flat and out-of-focus areas to the lowest layer covering
them, and only blown or crushed areas SHALL lose on exposure. An upper layer with
real detail SHALL win unless clearly less sharp. With Seamless on, broad tone SHALL
come from the lowest covering layer and detail from the sharpest, and gains SHALL
correct colour without changing overall brightness. Gains SHALL be measured over
flat areas. Panorama SHALL still match exposure. This is an approximation tuned
against observed Photoshop output.

#### Scenario: A flat upper background is dropped [abl_flat_base]

- **WHEN** a layer with a detailed subject on a flat background is stacked over a bright flat base
- **THEN** its mask hides the flat background and keeps the subject

#### Scenario: Upper detail survives busy texture [abl_upper_detail]

- **WHEN** a detailed subject is stacked over fine busy texture
- **THEN** the subject's mask reveals it

#### Scenario: Tone follows the base [abl_base_tone]

- **WHEN** a dark subject is stacked over a bright base with Seamless on
- **THEN** the merged subject's broad tone follows the base and its detail survives

### Requirement: Stack Images colour and focus edges

With Seamless on, Stack Images SHALL take only broad brightness from the lowest
covering layer; colour SHALL stay with the layer chosen for each area. Focus
selection SHALL stop at edges of the layer being scored, so a subject's detail does
not select the flat background beside it. This is an approximation tuned against
observed Photoshop output.

#### Scenario: The subject keeps its colour [abl_subject_colour]

- **WHEN** a red detailed subject is stacked over a blue base with Seamless on
- **THEN** the merged subject stays red

#### Scenario: No halo beside the subject [abl_focus_edge]

- **WHEN** a detailed subject on a flat background is stacked over a flat base
- **THEN** the mask all but hides the background right beside the subject
