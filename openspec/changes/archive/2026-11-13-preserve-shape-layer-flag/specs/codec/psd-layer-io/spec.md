# Spec Delta

## ADDED Requirements

### Requirement: Layer record pixel-data-irrelevant flag round-trips

The codec SHALL read bit 4 (`0x10`, "pixel data irrelevant to the appearance of
the document") of each modeled PSD and PSB layer record's flags into the layer
model and SHALL write that stored value back into the record. A section-divider
record is not a modeled layer and SHALL continue to be written with flags `0`.
When bit 4 is written, the record SHALL also set bit 3 (`0x08`, "flags valid")
so Photoshop treats bit 4 as meaningful; the writer SHALL leave bit 3 clear for
a layer whose stored bit 4 is clear, so documents whose layers never carry bit 4
serialize byte-for-byte as before. An unchanged read-save-read SHALL preserve
bit 4 for both set and clear values and SHALL leave the layer's other modeled
attributes unchanged; the remaining flag bits keep the writer's current behavior
(bit 3 is not preserved on layers without bit 4). The codec SHALL NOT infer the
bit from the layer's vector blocks: a layer read with bit 4 clear SHALL be
written with bit 4 clear even when it carries a vector mask, vector fill,
stroke, or origination block. The flag SHALL be absent at its default so
documents whose layers never carry it serialize byte-for-byte as before. An
independent PSD decoder SHALL classify a saved layer that carries bits 3 and 4
and shape vector blocks as a shape layer, and a vector-masked fill layer whose
bit is clear as a fill layer.

#### Scenario: Reading records the flag for both values [slf_read]

- **WHEN** `vector_fill.psd` is read
- **THEN** its `Shape` layer has the flag set and its `Base` layer has the flag clear

#### Scenario: Saving preserves a set flag in PSD and PSB [slf_round_trip]

- **WHEN** the document read from `vector_fill.psd` is written and read back as PSD and as PSB
- **THEN** the PSD reconstruction equals the input, the PSB reconstruction equals the input once the expected container flag is normalized, and its `Shape` layer still has the flag set

#### Scenario: A clear flag is not inferred from vector blocks [slf_clear_stays_clear]

- **WHEN** `vector_mask.psd`, whose vector-masked fill layers carry bit 4 clear, is read, written, and read back as PSD and as PSB
- **THEN** those layers still have the flag clear

#### Scenario: An independent decoder still sees a shape [slf_oracle_shape]

- **WHEN** the written copy of `vector_fill.psd` is opened in `psd-tools`
- **THEN** `Shape` is reported as a `shape` layer, `Base` as a `pixel` layer, and the shape record's raw flags are `0x18` (bits 3 and 4 set)

#### Scenario: An independent decoder still sees fill layers [slf_oracle_fill]

- **WHEN** the written copy of `vector_mask.psd` is opened in `psd-tools`
- **THEN** its vector-masked solid-color layers are still reported with the exact kind `solidcolorfill`

#### Scenario: Documents without the flag serialize unchanged [slf_default_bytes]

- **WHEN** a document whose layers all have bit 4 clear is written
- **THEN** its bytes are identical to the existing golden output

### Requirement: Authored shape layers carry the flag

A shape layer created by the shape tools as a solid-color fill clipped by an
authored vector mask SHALL be written with bits 3 and 4 (`0x18`) set. An
independent PSD decoder SHALL classify the saved layer as a shape layer. Other
fill layers and raster layers with vector masks SHALL NOT gain the flag.

#### Scenario: A drawn shape opens as a shape [slf_authored]

- **WHEN** a rectangle is drawn in Shape mode and the document is saved
- **THEN** `psd-tools` reports the layer as a `shape` layer and its record flags are `0x18`

#### Scenario: Ordinary fills and vector-masked pixels do not gain the flag [slf_no_gain]

- **WHEN** an ordinary solid-color fill layer created with `add_solid_fill` and a pixel layer carrying a vector mask with no fill adjustment are written and read back
- **THEN** both layers keep the pixel-data-irrelevant flag clear
