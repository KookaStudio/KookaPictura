# Spec Delta

## MODIFIED Requirements

### Requirement: Place imports supported raster images as an embedded smart object

The application SHALL expose `PictureView::place_image(path: &QString) -> QString`.
On success it SHALL append a raster layer through
`pictura_render::add_raster_layer_from_rgba`, convert that layer with the existing
`pictura_render::convert_to_smart_object`, release the raster proxy so the layer
renders from its embedded source, apply the canvas fit required by the
`compositing/smart-object-layer-actions` capability, recomposite, record exactly
one history state labelled `"Place"`, and return the new layer's path. The
embedded smart-object payload SHALL be authored at the raster's native pixel
dimensions and SHALL preserve the decoded transparency as a merged alpha plane,
so the rendered layer, Reset Transform, Rasterize, and Export Contents all see
the original alpha. Any refusal SHALL return an empty string and record no
history state. `File > Place…` SHALL offer an `Images (…)` filter. A
PSD/PSB file SHALL continue through `place_smart_object` and MUST NOT be routed
through Qt.

#### Scenario: A supported image is placed as a smart object

- **WHEN** `place_image` is called on a supported raster file with a document open
- **THEN** it returns the new layer's path, a new topmost layer carrying an embedded smart object exists, and exactly one `"Place"` history state is added

#### Scenario: The placed layer renders from its embedded native source

- **WHEN** a solid-color image is placed into a document
- **THEN** the composite shows the image's pixels within the placed layer's rect from the embedded source, and the raster proxy is released

#### Scenario: A transparent raster keeps its alpha [place_image_alpha]

- **WHEN** a raster with transparent and opaque pixels is placed
- **THEN** the placed layer's composite and its embedded source preserve the transparent and opaque alpha values, including after Reset Transform and Rasterize

#### Scenario: A refusal records nothing

- **WHEN** `place_image` is called with no open document or a missing, undecodable, or over-budget file
- **THEN** it returns an empty string, the document is unchanged, and no history state is added

#### Scenario: PSD stays native for Place

- **WHEN** `File > Place…` is given a `*.psd`/`*.psb` file
- **THEN** it routes through `place_smart_object` and not through the image decode edge
