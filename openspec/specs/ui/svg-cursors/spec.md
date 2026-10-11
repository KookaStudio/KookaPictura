# svg-cursors Specification

## Purpose
An original SVG cursor per implemented tool, resolved by id with catalogue hotspots applied to the active tool.
## Requirements
### Requirement: SVG cursor per tool
The system SHALL provide an original SVG cursor for each implemented tool under
`assets/cursors/`, named by the tool's `tool.<tool>` id, and SHALL expose
`QCursor cursor(const QString& id)` that resolves it to a non-null cursor.

#### Scenario: A known cursor resolves
- **WHEN** `cursor(id)` is called for a bundled tool cursor id
- **THEN** it returns a non-null `QCursor`

#### Scenario: An unknown cursor is null
- **WHEN** `cursor(id)` is called for an id with no asset
- **THEN** it returns a default/null cursor without crashing

### Requirement: Cursor hotspots

Each SVG cursor SHALL define an action hotspot, and the cursor art SHALL place
its action point at that hotspot. The hotspot SHALL be resolved from the frozen
tool catalogue, so each tool uses its documented action point: crosshair-style
selection, crop, path, shape, type, 3D and rotate tools at the centre
`(12,12)`; the Move tool at its compound cursor's arrowhead tip `(2,2)`;
brush-like and sample tools (Eyedropper, Color Sampler, Ruler, Note,
brush, stamp, eraser, fill, blur, toning, healing) at their lower-left tip
`(2,22)`; pen tools at their upper-left nib `(2,2)`; Hand and Zoom at the
pointing finger `(9,2)`. Every hotspot SHALL lie inside `[0, 24) × [0, 24)`.

#### Scenario: Hotspot matches the artwork

- **WHEN** a tool cursor is created
- **THEN** the hotspot is the catalogue's action point for that tool, and the art places its action point there

#### Scenario: An unknown id falls back to the centre

- **WHEN** a cursor is requested for an id with no catalogue hotspot
- **THEN** it uses the centre `(12,12)`

### Requirement: Active tool applies its cursor
The system SHALL set the canvas cursor to the active tool's SVG cursor whenever
the active tool changes or a canvas is bound to the controller. The SVG cursor is
the fallback pointer appearance; a tool MAY additionally draw an in-canvas overlay
(such as the brush-size circle) on top of it, and the presence of an overlay SHALL
NOT change the tool's resolved cursor or hotspot.

#### Scenario: Switching tools changes the cursor
- **WHEN** the active tool changes
- **THEN** the canvas cursor becomes that tool's SVG cursor
#### Scenario: An overlay does not replace the cursor [lsc_overlay_keeps_cursor]

- **WHEN** a tool draws an in-canvas overlay such as the brush-size circle
- **THEN** the tool's SVG cursor remains the pointer appearance and the overlay
  is drawn in addition

### Requirement: Base tool-arrow fill

Every cursor built on the workspace default arrow SHALL fill that arrow black
(`#202020`) beneath its white outline, matching `cursor.workspace.svg`. Any other
art compounded in the same cursor SHALL keep the standard white body fill, and
the arrow SHALL keep the white halo used for legibility on dark canvases. The
arrow geometry SHALL be the same path in every cursor that uses it.

#### Scenario: A tool cursor's arrow is black [svg_cursor_arrow_black]

- **WHEN** a cursor that compounds the workspace arrow (the Move, lasso, patch,
  or content-aware move cursors and `cursor.moveSelection`) is created
- **THEN** its arrow is filled black with the white halo, and the remaining art
  keeps the standard white fill

