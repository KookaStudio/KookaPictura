# filter-app-ui Specification

## Purpose
The PictureView filter command, filter-kind mapping and defaults, and the filter dock controls.

## Requirements

### Requirement: PictureView filter command

`PictureView` MUST expose `apply_filter(kind: &QString) -> bool`. It SHALL apply the mapped filter to the topmost pixel layer — the last layer in the bottom-first `doc.layers` for which `adjustment.is_none() && !is_group` — using the active selection as the mask, then recomposite and emit `changed`. With no active selection it MUST apply over the full frame. It MUST return false when there is no document or when there is no pixel layer.

#### Scenario: Applies to the topmost pixel layer

- **WHEN** a document has several pixel layers and `apply_filter` is called
- **THEN** the last bottom-first pixel layer (skipping adjustment and group layers) is the one filtered

#### Scenario: A selection confines the change

- **WHEN** an active selection covers part of the document and `apply_filter` is called
- **THEN** selected pixels change and unselected pixels are unchanged after recompositing

#### Scenario: No selection applies full-frame

- **WHEN** `apply_filter` is called with no active selection
- **THEN** the filter applies to the whole layer rect

#### Scenario: No document or no pixel layer returns false

- **WHEN** `apply_filter` is called with no document, or with a document that has no pixel layer
- **THEN** it returns false and the document is unchanged

### Requirement: Filter-kind mapping and defaults

The command SHALL map each recognized `kind` to a concrete `pictura_filters::Filter` with fixed defaults: `gaussian-blur` → `GaussianBlur { radius: 5.0 }`, `box-blur` → `BoxBlur { radius: 3 }`, `motion-blur` → `MotionBlur { angle: 0.0, distance: 15 }`, `median` → `Median { radius: 2 }`, `despeckle` → `Despeckle`, `sharpen` → `Sharpen`, `sharpen-more` → `SharpenMore`, `unsharp-mask` → `UnsharpMask { amount: 150.0, radius: 1.0, threshold: 0 }`, `add-noise` → `AddNoise { amount: 25.0, distribution: Uniform, monochromatic: false, seed: 1 }`, `dust-and-scratches` → `DustAndScratches { radius: 1, threshold: 0 }`, `extrude` → `Extrude { kind: Blocks, size: 30, depth: 30.0, level_based: true, solid_front: false, mask_incomplete: false }`, `tiles` → `Tiles { count: 10, offset: 10, fill: BackgroundColor, foreground: [0, 0, 0], background: [255, 255, 255] }`, `trace-contour` → `TraceContour { level: 128, edge: Lower }`, `wind` → `Wind { method: Wind, from_right: true }`, and `smart-sharpen` → `SmartSharpen { amount: 100.0, radius: 1.0, reduce_noise: 0.0, remove: GaussianBlur, angle: 0.0 }`. An unrecognized `kind` SHALL map to no filter and the command MUST return false without changing the document.

#### Scenario: Each known kind maps to its default filter

- **WHEN** `apply_filter` is called with each supported kind
- **THEN** the corresponding `Filter` with the listed defaults is applied

#### Scenario: Unknown kind is refused

- **WHEN** `apply_filter` is called with an unrecognized kind
- **THEN** it returns false and no layer is modified

### Requirement: Filter dock controls

The Qt dock MUST provide a filter combo box listing the supported kinds and an "Apply Filter" button that calls `apply_filter` with the combo's current kind. The image MUST refresh when the command succeeds.

#### Scenario: The Apply Filter button runs the selected filter

- **WHEN** a kind is chosen in the combo and "Apply Filter" is clicked
- **THEN** `apply_filter` runs with that kind and the displayed image refreshes

### Requirement: Filter confinement self-test

The headless `--self-test` MUST load a layered PSD, select a proper subset of the document, apply a filter to the topmost pixel layer confined by that selection, and assert that pixels outside the selection are unchanged while the selected region changed. It MUST exit non-zero on failure and MUST clean up before the remaining checks.

#### Scenario: Filtering is confined to the selection

- **WHEN** `--self-test` runs against a layered PSD
- **THEN** the selected region changes, pixels outside it are unchanged, and the run exits 0

#### Scenario: A failure exits non-zero

- **WHEN** the filtered result differs outside the selection or does not change inside it
- **THEN** the self-test exits non-zero

### Requirement: Artistic filter family exposure

The `Filter` menu SHALL contain an `Artistic` submenu listing all fifteen CS6 Artistic filters in CS6 order (`Colored Pencil`, `Cutout`, `Dry Brush`, `Film Grain`, `Fresco`, `Neon Glow`, `Paint Daubs`, `Palette Knife`, `Plastic Wrap`, `Poster Edges`, `Rough Pastels`, `Smudge Stick`, `Sponge`, `Underpainting`, `Watercolor`). Each entry SHALL open its parameter dialog, map to the corresponding engine kind, and be enabled exactly when the active layer is a filter target; a filter with no parameters SHALL apply directly.

#### Scenario: Every Artistic entry opens its dialog and applies

- **WHEN** each `Filter ▸ Artistic` entry is chosen on a document with an editable pixel layer
- **THEN** it maps to its engine kind and either opens the parameter dialog or applies directly, and the layer's colour planes change

#### Scenario: Disabled without a filter target

- **WHEN** no editable pixel layer is active
- **THEN** the Artistic entries are disabled

### Requirement: New filter-kind mapping

The filter mapping SHALL additionally recognise `lighting-effects` (19 slots, default Spot light), `diffuse` (one mode slot, default Normal), and `glowing-edges` (width, brightness, smoothness; defaults 2, 6, 1), each with an empty slot list producing those defaults and a non-empty list of the wrong length refused. These kinds SHALL drive `Filter ▸ Render ▸ Lighting Effects`, `Filter ▸ Stylize ▸ Diffuse`, and `Filter ▸ Stylize ▸ Glowing Edges`.

#### Scenario: The new kinds resolve and default

- **WHEN** each new kind is resolved with an empty slot list
- **THEN** the corresponding `Filter` with the listed defaults is produced

#### Scenario: Wrong arity is refused

- **WHEN** a new kind is resolved with a slot list of the wrong length
- **THEN** no filter is produced and the command returns false without changing the document

### Requirement: Convert for Smart Filters command

`Filter ▸ Convert for Smart Filters` SHALL convert the active raster layer into a smart object so that the layer's content is carried as an embedded source, while leaving the layer's pixels unchanged. It SHALL be enabled only when the active layer can be converted to a smart object, and on success it SHALL recomposite and record one history state named `Convert for Smart Filters`.

#### Scenario: Converting preserves the pixels and records one state

- **WHEN** `Convert for Smart Filters` is invoked on a convertible raster layer
- **THEN** the layer becomes a smart object with the same composited pixels and exactly one history state is recorded

#### Scenario: Refused when not convertible

- **WHEN** the active layer cannot be converted to a smart object
- **THEN** the command is disabled or refused and the document is unchanged

### Requirement: Parameter slider activates its value box

A filter parameter row that carries both a slider and a value box SHALL focus the
value box when the slider is clicked, so Up/Down (and typing) adjust the same
parameter without a separate click into the box.

#### Scenario: Clicking the slider focuses the box [fa_slider_focus]

- **WHEN** a parameter slider is clicked
- **THEN** its value box takes focus, so a following Up/Down changes the
  parameter
