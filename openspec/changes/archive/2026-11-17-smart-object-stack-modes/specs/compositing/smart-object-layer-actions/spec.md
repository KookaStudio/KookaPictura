# Spec Delta

## MODIFIED Requirements

### Requirement: Rasterizing materializes the object content into layer channels

On success the operation SHALL materialize the object's rendered content into
the layer's pixel channels and SHALL set `layer.smart_object` to `None`. When
the layer already has a color channel (`id == 0`) and no eligible active stack
mode applies, that existing raster proxy IS the content, so the operation SHALL
leave every pixel channel byte-for-byte unchanged and SHALL NOT decode the
payload. When a layer with a color channel carries an eligible active stack mode
(an embedded payload with at least two visible frames and every enabled Smart
Filter decodes), the operation SHALL materialize the reduced stack source into
channels `0..mode.color_channels()` plus a `-1` alpha channel. When a layer with
a color channel carries a stack mode whose stack source does not apply, the
operation SHALL keep the proxy byte-for-byte. When the layer has no color
channel, the operation SHALL decode the embedded payload through the existing
embedded-source render path, scale the decoded source into the layer's `rect`,
and write channels `0..mode.color_channels()` plus a `-1` alpha channel; when
that layer carries a stack mode, the source is the reduced stack render when the
reduced stack source applies, otherwise the existing embedded source. The
operation SHALL NOT change the layer's name, `rect`, blend, opacity, fill, mask,
or position.

#### Scenario: A proxy is left unchanged
- **WHEN** a smart-object layer with a color channel and no eligible active stack mode is rasterized
- **THEN** the layer's pixel channels are byte-for-byte the same as before

#### Scenario: A source-only layer materializes from the payload
- **WHEN** a smart-object layer with no color channel but a decodable embedded payload is rasterized
- **THEN** the layer's color channels and `-1` alpha hold the decoded source scaled into the layer `rect`

#### Scenario: A stack-mode proxy bakes the reduction
- **WHEN** a proxied smart-object layer with an eligible active stack mode is rasterized
- **THEN** its pixel channels hold the reduced stack source and the layer no longer carries a smart object

#### Scenario: A non-applicable stack mode keeps the proxy
- **WHEN** a proxied smart-object layer carries a stack mode for which the reduced stack source does not apply
- **THEN** its pixel channels are byte-for-byte the same as before

#### Scenario: The smart object is cleared
- **WHEN** rasterizing succeeds
- **THEN** `layer.smart_object` is `None`

#### Scenario: Layout fields are untouched
- **WHEN** a layer is rasterized
- **THEN** its name, `rect`, blend, opacity, fill, mask, and position are unchanged
