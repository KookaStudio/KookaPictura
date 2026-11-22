# tools/non-destructive-crop Specification

## Purpose
The Crop tool's `Delete Cropped Pixels` off path and `Image > Reveal All`: the
pixels outside the canvas stay in the layer rasters, and Reveal All grows the
canvas back over all content.

## ADDED Requirements

### Requirement: Retained pixels

With `Delete Cropped Pixels` off, committing a crop SHALL re-frame the canvas
without discarding any layer's pixels outside the new canvas. With it on, the
committed crop SHALL trim each pixel layer's channels and mask to the canvas.

#### Scenario: A non-destructive crop keeps the raster [ndc_retains]

- **WHEN** a crop is committed with `Delete Cropped Pixels` off
- **THEN** the layer rects extend outside the smaller canvas and their channel
  planes still hold the cropped-away pixels

#### Scenario: A destructive crop trims the raster [ndc_trims]

- **WHEN** a crop is committed with `Delete Cropped Pixels` on
- **THEN** each pixel layer's rect, channels, and mask are trimmed to the canvas

### Requirement: Reveal All command

`Image > Reveal All` SHALL be a command-catalogue row with a menu handler. It
SHALL grow the canvas to the union of the current canvas and every layer's
content, including group descendants, layer masks, and retained native stores,
without resampling. It SHALL record exactly one `Reveal All` history state and
clear the selection when the canvas changes.

#### Scenario: Reveal All restores a non-destructive crop [ndc_reveal_all]

- **WHEN** a 24 x 24 document with a filled block at (3, 2) is cropped to a
  5 x 4 box with `Delete Cropped Pixels` off, and `Image > Reveal All` runs
- **THEN** the canvas is 24 x 24 again and the block is back at (3, 2) with its
  pixel values unchanged

#### Scenario: Reveal All is one undo step [ndc_reveal_undo]

- **WHEN** Reveal All grows the canvas
- **THEN** one `Reveal All` state is recorded and undo returns to the cropped
  canvas

#### Scenario: Reveal All with nothing outside [ndc_reveal_noop]

- **WHEN** every layer's content lies inside the canvas
- **THEN** Reveal All changes nothing and records no history state

#### Scenario: A destructive crop cannot be revealed [ndc_reveal_after_delete]

- **WHEN** a crop is committed with `Delete Cropped Pixels` on and Reveal All
  runs
- **THEN** the canvas stays at the crop size because the pixels were discarded
