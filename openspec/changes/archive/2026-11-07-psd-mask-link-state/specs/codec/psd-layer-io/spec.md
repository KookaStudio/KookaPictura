# Spec Delta

## ADDED Requirements

### Requirement: Raster mask link state survives PSD read and save

PSD and PSB raster-mask flags SHALL be preserved in their on-disk
representation: bit 0 clear means linked, and bit 0 set means unlinked. Reading
or saving either state SHALL preserve the mask's document-space rectangle and
coverage without adding or subtracting the owning layer's offset. An unchanged
read-save-read SHALL preserve the link bit and the other mask-flag bits. A link
toggle followed by save and read SHALL change only bit 0 of the mask flags,
leaving other mask attributes unchanged. Independent decoder checks SHALL
verify the written bit values for both states.

#### Scenario: A linked external mask remains linked [pml_codec_linked]

- **WHEN** an independently authored PSD with mask bit 0 clear and different non-zero layer and mask origins is read, saved unchanged, and reopened
- **THEN** its mask reads as linked before and after saving, with the same flags, document-space rectangle, and coverage

#### Scenario: An unlinked external mask remains unlinked [pml_codec_unlinked]

- **WHEN** an independently authored PSD with mask bit 0 set and different non-zero layer and mask origins is read, saved unchanged, and reopened
- **THEN** its mask reads as unlinked before and after saving, with the same flags, document-space rectangle, and coverage

#### Scenario: Link edits persist in both document formats [pml_codec_toggle]

- **WHEN** a raster mask is linked and unlinked in turn, saving and reopening as PSD and PSB after each edit
- **THEN** each reopened state matches the edit, only bit 0 of its flags changes between states, and its other attributes are preserved

#### Scenario: An independent decoder reads the saved flags [pml_codec_oracle]

- **WHEN** files saved with linked and unlinked raster masks are opened in an independent PSD decoder
- **THEN** their raw mask bit 0 values are clear and set respectively, and the decoded mask rectangles and coverage agree with the saved masks
