## MODIFIED Requirements

### Requirement: Auto-Blend availability

Edit > Auto-Blend Layers SHALL be available only for an RGB or Grayscale document
with at least two eligible selected layers and no pending modal transform. It
MUST be disabled when a selected layer is the Background, a Smart Object, a video
or 3D layer, or a clipped layer; a Smart Object outside the selection MUST NOT
disable it. A layer SHALL count as a video or 3D layer when it carries
Photoshop's `PxSD` pixel-source block, the block the PSD specification marks as
raw data for 3D or video layers. Groups, adjustment and fill layers, type and
shape layers, hidden layers and empty layers in the selection SHALL be skipped
without a mask and SHALL NOT count as eligible. Locked layers SHALL be processed.
The skip and clipped-layer rules are inferred.

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

#### Scenario: Video and 3D layers are refused [abl_video_3d]

- **WHEN** a layer carrying the `PxSD` pixel-source block is selected beside two raster layers
- **THEN** Auto-Blend Layers is unavailable, a plan is empty, and the document is unchanged
