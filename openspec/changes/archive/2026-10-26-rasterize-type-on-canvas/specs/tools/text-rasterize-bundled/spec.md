## ADDED Requirements

### Requirement: Rasterizing type keeps its on-canvas render

Rasterize Type, Rasterize Layer and Rasterize All Layers SHALL turn a type layer
that carries its rendered colour and alpha planes into a pixel layer with the
same rect and colour planes. The alpha SHALL bake the composite-time text gamma,
so the document composite does not change by more than rounding. A type layer
without rendered planes SHALL be painted over its rect by the bundled backend,
as the compositor draws it. Rasterizing SHALL drop `TySh`, the type tool, the
applied style names and overrides, and the transparency and pixel locks. The
pixels SHALL NOT come from a second, Qt-side text render.

#### Scenario: A transformed and moved type layer keeps its size and place [rtoc_transformed]

- **WHEN** a type layer scaled by Free Transform and then moved is rasterized
- **THEN** its rect and colour planes are the on-canvas render, its alpha is the baked text gamma, and the document composite is unchanged within 2 levels

#### Scenario: Rasterize Type through the app keeps the composite [rtoc_app]

- **WHEN** `Rasterize Type` runs on a Free Transformed type layer in the app
- **THEN** one `Rasterize Type` state is recorded, the layer is no longer type, its rect is unchanged, and the sampled composite is unchanged within 2 levels

#### Scenario: The raster is unlocked and styleless [rtoc_unlocked]

- **WHEN** a type layer with its type locks and an applied paragraph style is rasterized
- **THEN** the layer has no locks and no applied style

#### Scenario: A colour font rasterizes as shown [rtoc_color_font]

- **WHEN** a type layer set in a COLR colour font is rasterized
- **THEN** the raster shows the same glyphs in the text colour as the canvas did, with no palette layers or garbage tiles

## REMOVED Requirements

### Requirement: The Rasterize Type command can render through the Qt backend

**Reason**: The Qt render ignored the type transform, the font family, and the
engine's layout, and painted colour-font palette layers. Rasterized type did not
match the canvas (#962).

**Migration**: Rasterize Type keeps the on-canvas render; see "Rasterizing type
keeps its on-canvas render".
