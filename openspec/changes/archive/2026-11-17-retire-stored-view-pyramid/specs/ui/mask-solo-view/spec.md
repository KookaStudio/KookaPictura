## MODIFIED Requirements

### Requirement: Mask solo presents through the view pyramid

The mask display SHALL be cropped from a tile-pyramid level exactly as the
composite is (`ui/document-canvas`), exposing the same level count and per-level
sizes, selecting the level from the current zoom, so a repaint after a pan or
zoom SHALL NOT resample a full-resolution image. The composite tile source
SHALL NOT be replaced by the mask source; solo SHALL select mask tiles while
other consumers continue to read composite tiles.

#### Scenario: The mask crops a level [msv_level]

- **WHEN** a mask is soloed and the canvas repaints
- **THEN** the presented pixels come from a tile-pyramid level crop of the mask
  plane at the zoom-selected level, with the level count and sizes unchanged

#### Scenario: Leaving solo restores the composite pyramid [msv_level_restore]

- **WHEN** the solo mode ends
- **THEN** subsequent crops show the document composite again, with no rebuild of
  the document
