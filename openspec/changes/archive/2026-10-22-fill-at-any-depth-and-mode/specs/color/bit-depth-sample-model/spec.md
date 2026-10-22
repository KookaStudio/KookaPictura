# Spec Delta

## MODIFIED Requirements

### Requirement: The typed store round-trips the source bytes exactly

For an unedited 16/32-bit document the write path SHALL encode the retained
typed samples back to the same bytes the read path decoded (byte-identical for
big-endian `u16` and IEEE-754 big-endian `f32`), so the existing source-depth
round-trip and its `psd-tools` oracle are unchanged. A plane edited in the
8-bit working copy SHALL continue to be widened from its 8-bit bytes as before;
a plane edited in the retained native store SHALL be re-encoded at the source
depth from the store's own samples.

#### Scenario: Encode inverts decode

- **WHEN** an arbitrary native sample image is decoded to samples and re-encoded
- **THEN** the bytes are identical to the input for both depth 16 and depth 32

#### Scenario: The depth oracle is unchanged

- **WHEN** the existing depth and color-mode oracles run after this change
- **THEN** they pass with no golden change

#### Scenario: A natively edited plane is re-encoded at the source depth

- **WHEN** a depth-16 document's layer is edited in its retained native store
  and saved
- **THEN** the written layer plane holds the store's samples and not the 8-bit
  widening of the layer's 8-bit channels
