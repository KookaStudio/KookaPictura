# text-rasterize-bundled Specification

## Purpose
Rasterizes type layers with the bundled pure-Rust glyph rasterizer, including Rasterize Layer/All and a dedicated command.

## Requirements

### Requirement: A bundled-font, pure-Rust glyph rasterizer

`pictura-render` SHALL bundle one permissively-licensed font and implement
`pictura_core::Rasterizer` over it, so a `RasterRequest` for a glyph yields a
`GlyphMask` whose coverage length is `width * height`, with no Qt and no C
dependency. The bundled font and its license SHALL be recorded in the
repository.

#### Scenario: A glyph rasterizes to coverage

- **WHEN** the bundled rasterizer is asked for a glyph present in the bundled font
- **THEN** it returns a non-empty `GlyphMask` whose coverage length equals `width * height`

#### Scenario: An absent glyph is a no-op

- **WHEN** the requested glyph index has no rasterizable outline
- **THEN** the rasterizer returns `None` and no panic

### Requirement: A type layer materializes into pixels

`pictura-render` SHALL expose `render_text_layer(document, path)` that shapes and
lays out the layer's `TypeTool` text and style with the bundled font, paints the
glyph coverage in the style's fill colour into the layer's colour and alpha
channels, removes the `TySh` block, and clears the type-tool view. It SHALL
return `false` without mutating when the layer has no type tool or style, when
its rect has no area, or when no glyph rendered, so the caller records exactly
one history state on success.

#### Scenario: A type layer becomes a pixel layer

- **WHEN** a type layer with a style is rendered
- **THEN** its channels contain non-empty coverage, `type_tool` is cleared, and the `TySh` block is gone

#### Scenario: A missing style is a no-op

- **WHEN** a layer has a type tool but no decoded style
- **THEN** `render_text_layer` returns `false` and the channels and blocks are unchanged

#### Scenario: Provenance records the substitution

- **WHEN** provenance is produced for a requested family other than the bundled one
- **THEN** it records the requested family, the bundled resolved family, the bundled font hash, and the backend

### Requirement: Rasterize Layer handles type layers

`PictureView::rasterize_layer` SHALL rasterize a type layer through the bundled
backend: it SHALL attempt `render_text_layer` first and, when that renders,
recomposite, record exactly one `Rasterize Type` history state, and return true.
For a non-type layer it SHALL fall back to the fill-content rasterizer and
record as before. A layer that neither path can rasterize SHALL return false and
record nothing.

#### Scenario: A type layer rasterizes through the command

- **WHEN** `rasterize_layer` is called on a type layer with a decoded style
- **THEN** its text is painted into its channels, it is no longer a type layer, and one `Rasterize Type` state is recorded

#### Scenario: A fill layer still rasterizes as before

- **WHEN** `rasterize_layer` is called on a decodable fill-content layer
- **THEN** the fill is baked and one `Rasterize Layer` state is recorded

#### Scenario: An un-rasterizable layer records nothing

- **WHEN** `rasterize_layer` is called on a layer that has neither a decodable fill nor a type tool
- **THEN** it returns false and records no history state

### Requirement: Rasterize All Layers covers type layers

`pictura-render` SHALL expose `rasterize_all_layers(document) -> usize` that
rasterizes every flattened layer through the fill-content path or, failing that,
the bundled text path, and returns how many were rasterized. The command SHALL
record exactly one history state when the count is positive.

#### Scenario: A fill layer and a type layer both rasterize

- **WHEN** a document holds one fill-content layer and one type layer
- **THEN** `rasterize_all_layers` returns 2, the fill is baked, and the type layer is materialized

#### Scenario: Already rasterized layers are skipped

- **WHEN** no layer is fill content or a type layer
- **THEN** `rasterize_all_layers` returns 0

### Requirement: A dedicated Rasterize Type command

`PictureView::rasterize_type(path)` SHALL materialize the type layer at `path`
through the bundled backend, recomposite, record exactly one `Rasterize Type`
history state, and return true; it SHALL return false and record nothing for a
layer that is not a type layer. The `Layer > Rasterize > Type` command SHALL be
enabled only when the selected layer is a type layer.

#### Scenario: A type layer rasterizes through the Type command

- **WHEN** `rasterize_type` is called on a type layer with a decoded style
- **THEN** its text is painted into its channels, the layer is no longer a type layer, and one `Rasterize Type` state is recorded

#### Scenario: A non-type layer is refused

- **WHEN** `rasterize_type` is called on a plain pixel layer
- **THEN** it returns false and records no history state

#### Scenario: The command is gated to type layers

- **WHEN** the selected layer is not a type layer
- **THEN** the `layer.rasterize.type` action is disabled

### Requirement: The Qt text helper is directly testable

`pictura-render` SHALL expose `materialize_text_rgba(document, path, rgba)` that
replaces a layer's colour and alpha channels from a packed RGBA buffer of the
layer-rect size, drops `TySh`, and clears the type tool, returning false without
mutating for a wrong-size buffer or a missing layer.

#### Scenario: A wrong-size buffer is refused

- **WHEN** `materialize_text_rgba` is called with a buffer that is not `width*height*4` bytes
- **THEN** it returns false and the layer is unchanged

### Requirement: The bundled text backend is one pure-Rust stack

The bundled text backend SHALL depend only on pure-Rust font crates: `rustybuzz`
for shaping, `ttf-parser` for glyph count, character lookup, and line metrics,
and `swash` for rasterization. It SHALL NOT keep a second rasterizer or metrics
library that nothing uses. The backend SHALL remain free of any C dependency.

#### Scenario: No redundant font library

- **WHEN** the crate's dependency list is inspected
- **THEN** shaping is `rustybuzz`, rasterization is `swash`, metrics/lookup are `ttf-parser`, and no other font rasterizer or metrics crate is present

#### Scenario: Metrics still resolve from the bundled face

- **WHEN** the backend needs the glyph count, the glyph id for a character, or the ascent
- **THEN** each is answered by the bundled `ttf-parser` face

#### Scenario: Text rendering is unchanged

- **WHEN** the existing shape, raster, render, kerning, and subpixel tests run
- **THEN** they pass with the same assertions

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
