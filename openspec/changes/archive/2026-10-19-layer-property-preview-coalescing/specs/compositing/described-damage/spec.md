# Spec Delta

## MODIFIED Requirements

### Requirement: A bounded pixel operation refreshes only its region

The app SHALL refresh only a bounded dirty rectangle, through the same regional
composition path used by visibility toggles and move previews, for a
pixel-changing operation whose effect it can bound: a filter or adjustment
applied to a pixel layer with a bounded destination rectangle, a blend mode,
opacity, or fill-opacity change (including a live preview and the commit that
finishes it) on a layer whose rectangle bounds its effect (a canvas-sized
rectangle is allowed and uses the region path), a clipboard clear or paste on
a bounded rectangle, a paint stroke commit or cancel on the stroke's changed
rectangle, and a layer move or free-transform commit on the union of the source
and destination rectangles. The refreshed canvas MUST be byte-identical to a
full recomposite of the document, every pixel outside the rectangle MUST keep
its previous value, and the operation MUST NOT run the full-document recomposite
path. A layer whose rectangle equals the canvas MAY composite that rectangle
through the region path; its cost then equals a full composite, but the region
path and its signals are still used.

#### Scenario: A filter on a bounded layer updates only its rectangle

- **WHEN** a filter is applied to a pixel layer whose rectangle is smaller than the document
- **THEN** only that rectangle is composited, every pixel outside it keeps its previous value, and the result equals a full recomposite

#### Scenario: An opacity change on a bounded layer updates only its rectangle

- **WHEN** a layer's opacity or blend mode is changed and the layer has a bounded rectangle
- **THEN** only that rectangle is composited and the result equals a full recomposite

#### Scenario: A live opacity preview updates only its rectangle [dd_preview_region]

- **WHEN** a layer's opacity or fill is previewed during a drag and the layer has
  a bounded rectangle
- **THEN** only that rectangle is composited, every pixel outside it keeps its
  previous value, and the result equals a full recomposite

#### Scenario: A canvas-sized layer uses the region path [dd_canvas_region]

- **WHEN** a full-canvas pixel layer's opacity or fill changes
- **THEN** the region path is used even though the rectangle equals the canvas,
  and the full-document recomposite path is not run

#### Scenario: A paste updates only the pasted rectangle

- **WHEN** a clipboard paste writes an image at a bounded destination rectangle
- **THEN** only that rectangle is composited and the result equals a full recomposite

#### Scenario: A paint commit updates only the stroke's rectangle

- **WHEN** a completed or cancelled paint stroke is committed
- **THEN** only the stroke's changed rectangle is composited and the result equals a full recomposite

#### Scenario: A transform commit updates the union of source and destination

- **WHEN** a layer move or free transform is committed
- **THEN** the union of the layer's source and destination rectangles is composited and the result equals a full recomposite
