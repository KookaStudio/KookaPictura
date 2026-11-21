## MODIFIED Requirements

### Requirement: Filter Gallery dialog

`Filter ▸ Filter Gallery…` SHALL open a dialog with a zoomable preview, the gallery filters as thumbnails grouped under collapsible category headers (Artistic, Brush Strokes, Distort, Sketch, Stylize, Texture, in CS6 order, each listing only filters with an engine kernel), a toggle that hides the thumbnails, OK and Cancel, a menu of every gallery filter, the selected effect's options, and an effect-layer list. The entry SHALL be enabled exactly when the active layer can take a filter.

#### Scenario: Categories follow CS6

- **WHEN** the gallery lists its categories
- **THEN** Artistic has 15 filters, Brush Strokes 8, Distort 3, Sketch 14, Stylize 9 (Diffuse, Emboss, Extrude, Find Edges, Glowing Edges, Solarize, Tiles, Trace Contour, Wind in that order), and Texture 6

#### Scenario: Thumbnails show the picture filtered

- **WHEN** the gallery opens
- **THEN** each thumbnail is rendered from the picture through its filter
