# mask-solo-view Specification

## Purpose

Show a layer mask on its own: an `Alt`-click of the mask thumbnail replaces the
canvas composite with the mask's grayscale channel, and a second click restores
it. The mode is a transient display state that never touches the document, and it
presents through the same view-pyramid level-crop path as the composite so zoom
and pan behave identically and a large document is never scaled full-resolution
per paint. Only the document canvas reads the mask; every other display consumer
keeps reading the composite.

## ADDED Requirements

### Requirement: The canvas renders the mask plane alone

When a raster layer mask is the solo target, the document canvas SHALL present
that mask's grayscale plane in place of the composite: each sample SHALL be a
uniformly grey, fully opaque pixel (the sample replicated across red, green, and
blue, alpha 255). Samples outside the mask's own rectangle SHALL use the mask's
own `default_color`, and a data-less mask SHALL be fully revealing white. A
disabled mask SHALL still render its stored coverage rather than the compositor's
reveal-all white, so the mask's raster stays visible while it is switched off (the
display shows the mask, not what the mask does to the layer). The mask plane SHALL
be presented at the document's size and position, so the canvas keeps the
document's dimensions and the same pan/zoom transform.
The mask SHALL be the sole canvas *source*: the Channels panel, the Histogram
panel, the Navigator thumbnail, the filter and adjustment previews, and every
save/export accessor SHALL continue to read the document composite. The mode
SHALL apply only to a raster layer mask; a row without a raster mask SHALL have
no solo display (a guarantee of the bridge, not of the panel gesture, which fires
only on a mask thumbnail that exists).

#### Scenario: A soloed mask shows its grayscale [msv_plane]

- **WHEN** a layer with a raster mask is soloed
- **THEN** the canvas draws the mask's samples as opaque greyscale instead of the
  composite

#### Scenario: The document size is unchanged [msv_plane_extent]

- **WHEN** a mask is soloed
- **THEN** the canvas still reports the document's width and height and the view
  transform is unchanged

#### Scenario: Off-rect and data-less samples render the mask's own extent [msv_plane_edges]

- **WHEN** a soloed mask's rectangle is smaller than the document, or the mask
  carries no pixels
- **THEN** off-rect pixels use the mask's `default_color` and a data-less mask is
  fully white

#### Scenario: A disabled mask shows its stored coverage [msv_plane_disabled]

- **WHEN** the soloed mask is disabled
- **THEN** the canvas draws the mask's stored coverage (black where it hides, grey
  where partial), not the fully-white compositor result

#### Scenario: Other consumers keep the composite [msv_plane_scoped]

- **WHEN** a mask is soloed
- **THEN** the Channels panel, Histogram panel, Navigator thumbnail, filter and
  adjustment previews, and the full-resolution `image()` all still report the
  document composite

#### Scenario: A row without a raster mask does not solo [msv_plane_none]

- **WHEN** the solo target path has no raster mask
- **THEN** the bridge reports no solo source and the canvas shows the composite

### Requirement: Mask solo presents through the view pyramid

The mask display SHALL be cropped from a view-pyramid level exactly as the
composite is (`ui/document-canvas`), exposing the same level count and per-level
sizes, selecting the level from the current zoom, so a repaint after a pan or
zoom SHALL NOT resample a full-resolution image. The stored composite pyramid
SHALL NOT be rebuilt or replaced by the solo mode.

#### Scenario: The mask crops a level [msv_level]

- **WHEN** a mask is soloed and the canvas repaints
- **THEN** the presented pixels come from a view-pyramid level crop of the mask
  plane at the zoom-selected level, with the level count and sizes unchanged

#### Scenario: Leaving solo restores the composite pyramid [msv_level_restore]

- **WHEN** the solo mode ends
- **THEN** subsequent crops show the document composite again, with no rebuild of
  the document

### Requirement: Mask solo reflects edits to the mask

While a mask is soloed, a change to the mask's coverage SHALL refresh the
displayed plane for the changed region through the same present path every other
edit uses, so painting, filling, or filtering the mask while viewing it shows the
new coverage without leaving the mode.

#### Scenario: Painting the mask updates the view [msv_live_edit]

- **WHEN** the mask is soloed and the user paints on it (the Alt-click makes the
  mask the edit target)
- **THEN** the presented mask region equals the mask's new coverage

#### Scenario: A full rebuild keeps the mode [msv_live_rebuild]

- **WHEN** a full recomposite runs while a mask is soloed
- **THEN** the canvas still shows the mask plane, not the composite

### Requirement: Mask solo is view-only

Mask solo SHALL be a transient display state: it SHALL record no history state,
SHALL NOT mutate the document's pixels, layer tree, visibility, or flags, and
SHALL NOT be serialized on save. The full-resolution `image()` accessor and any
export path SHALL continue to return the document composite while a mask is
soloed.

#### Scenario: Entering solo records no history [msv_no_history]

- **WHEN** a mask is soloed and then the composite is restored
- **THEN** the document's history length and current position are unchanged

#### Scenario: Layer visibility is untouched [msv_no_mutation]

- **WHEN** a mask is soloed and then restored
- **THEN** every layer's visibility, opacity, and mask enabled/linked flags are
  exactly as before

#### Scenario: Save sees the composite [msv_no_serialize]

- **WHEN** a mask is soloed and the full-resolution image is requested
- **THEN** it is the document composite, not the mask plane

#### Scenario: Removing the mask or its layer ends solo [msv_removed]

- **WHEN** the soloed mask is deleted, or its layer is deleted, grouped away, or
  the document is otherwise reset
- **THEN** the solo state is cleared and the composite is shown

#### Scenario: Switching documents ends solo [msv_reset]

- **WHEN** a mask is soloed and a different document is shown
- **THEN** the newly shown document carries no mask solo state

### Requirement: Mask solo follows the current layer

The solo view SHALL be tied to the current layer. Selecting a different layer
SHALL end the solo and restore the composite, whether the selection came from a
layer thumbnail, the row body, another layer's mask thumbnail, or any other
gesture. Toggling the soloed mask's own enabled state — **Enable Layer Mask** /
**Disable Layer Mask**, or a `Shift`-click of its thumbnail — SHALL NOT end the
solo, and the canvas SHALL keep showing the mask's stored coverage. A recomposite
triggered by such a toggle SHALL preserve the mode.

#### Scenario: Selecting another layer ends solo [msv_scope_layer]

- **WHEN** a mask is soloed and the user selects a different layer
- **THEN** the solo state is cleared and the canvas shows the composite

#### Scenario: Toggling the soloed mask keeps it soloed [msv_enable_keeps]

- **WHEN** the soloed mask is enabled or disabled (menu or `Shift`-click)
- **THEN** the canvas still shows the mask's stored coverage and the solo is not
  cleared
