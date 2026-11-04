## MODIFIED Requirements

### Requirement: A proxy-less type layer renders live in the compositor

The CPU compositor SHALL render a non-group layer that carries a `TypeTool` and
has no colour channel by shaping and rasterizing its text with the bundled
backend and compositing the result over the layer's canvas-clipped region with
the layer's opacity, mask, and blend mode, instead of painting the channel-less
rect. A type layer that carries a colour channel SHALL still composite from that
raster. When a proxy-less type layer's text paints no pixel, the compositor
SHALL leave the backdrop unchanged instead of painting the layer's rect.

#### Scenario: Proxy-less type renders coverage

- **WHEN** a document has a type layer with a style and no colour channel
- **THEN** its region composites non-empty text coverage rather than an opaque black rect

#### Scenario: A stored proxy is unchanged

- **WHEN** a type layer carries a colour channel
- **THEN** the compositor uses the channel and the live renderer is not invoked for it

#### Scenario: Type that paints nothing is transparent [tlc_no_ink]

- **WHEN** a proxy-less type layer's text renders no painted pixel
- **THEN** the composited backdrop under its rect is unchanged
