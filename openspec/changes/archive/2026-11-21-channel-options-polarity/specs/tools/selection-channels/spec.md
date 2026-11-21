# Spec Delta

## ADDED Requirements

### Requirement: Channel sense and PSD DisplayInfo

Every document-level extra channel SHALL carry a sense, either `Masked Areas` or `Selected Areas`. `Masked Areas` SHALL be the default for a channel with no stored sense. Under `Masked Areas` the channel plane SHALL equal the selection coverage: white is selected and black is protected. Under `Selected Areas` the plane SHALL store protection: white is protected and black is selected, so the selection coverage is `255 - plane` for every pixel.

The sense SHALL be stored in the PSD DisplayInfo resource (1077): kind 1 is `Masked Areas`, kind 0 is `Selected Areas`, and kind 2 is spot. A channel with no DisplayInfo entry SHALL be read as `Masked Areas`. Entries SHALL stay aligned with the extra-channel order: adding a channel SHALL append a `Masked Areas` entry when a resource exists or leave the channel defaulting to `Masked Areas`, deleting a channel SHALL remove its entry and shift the following entries, and rewriting one channel's kind SHALL preserve every other entry's color, opacity, and kind. A new alignment entry SHALL NOT be a default-constructed DisplayInfo entry, whose mode is `Selected Areas`. Saving and reopening a PSD/PSB SHALL preserve each channel's sense and plane, and psd-tools SHALL report the stored kind for each channel.

Changing a channel's sense SHALL preserve its loaded selection by flipping the plane and the kind together: `plane' = 255 - plane`.

#### Scenario: New extra channels default to Masked Areas [chan_sense_default]
- **WHEN** a document gains a new extra channel and no sense is set
- **THEN** its sense is `Masked Areas`, its plane is all zero (nothing selected), and it has no required DisplayInfo entry

#### Scenario: A Selected channel stores protection [chan_sense_selected_plane]
- **WHEN** canonical coverage 200 is saved into a channel whose sense is `Selected Areas`
- **THEN** every stored plane byte is 55 and loading the channel yields coverage 200

#### Scenario: The same plane reads opposite under each sense [chan_sense_inverse]
- **WHEN** the identical plane bytes P are interpreted under `Masked Areas` and under `Selected Areas`
- **THEN** the loaded coverage is P under `Masked Areas` and `255 - P` under `Selected Areas`

#### Scenario: A missing entry is Masked Areas [chan_sense_missing]
- **WHEN** a PSD channel has no DisplayInfo entry
- **THEN** it loads as a `Masked Areas` channel, its plane read as selection coverage

#### Scenario: The sense round-trips through PSD [chan_sense_roundtrip]
- **WHEN** a document with one Masked and one Selected channel is written and read back
- **THEN** both senses and both planes are unchanged

#### Scenario: psd-tools reports the kinds [chan_sense_psd_tools]
- **WHEN** psd-tools opens a written PSD with one Masked and one Selected channel
- **THEN** its DisplayInfo reports kind 1 and kind 0 for the two channels in order

#### Scenario: Deleting a channel shifts the following entries [chan_sense_delete_shift]
- **WHEN** the first of three extra channels is deleted
- **THEN** the remaining channels keep their senses and every other DisplayInfo field

#### Scenario: Adding a channel leaves the other senses [chan_sense_add_aligned]
- **WHEN** a document with a `Selected Areas` channel and an existing DisplayInfo resource gains a new channel
- **THEN** the Selected channel still loads the same selection and the new channel is `Masked Areas`

#### Scenario: Clearing channels clears their display info [chan_sense_clear]
- **WHEN** a mode conversion clears a document's extra channels and a new channel is added
- **THEN** the new channel is `Masked Areas`, with no stale kind from the cleared channels

#### Scenario: PDF preserve-editing fills a missing entry as Masked [chan_sense_pdf_default]
- **WHEN** the PDF preserve-editing path retains a subset of channels and a retained channel has no DisplayInfo entry
- **THEN** its sense is `Masked Areas`, not `Selected Areas`

#### Scenario: Toggling the sense preserves the selection [chan_sense_toggle]
- **WHEN** a channel's sense changes from `Masked Areas` to `Selected Areas`
- **THEN** its plane is complemented, its kind becomes 0, and the selection it loads is byte-for-byte unchanged

### Requirement: Sense-aware selection conversion

Saving a selection to an extra channel SHALL write the plane through that channel's sense: coverage unchanged under `Masked Areas`, and `255 - coverage` under `Selected Areas`. Saving into an existing channel SHALL combine the existing channel's canonical coverage with the incoming selection using the chosen operation, then write the result through the channel's sense. New channels created by New Channel or by Save Selection SHALL be `Masked Areas`.

Loading an extra channel SHALL derive canonical coverage through the channel's sense first, then apply the Load Selection `Invert` flag, then combine with the current selection using the chosen operation. Duplicate Channel and cross-document channel drags SHALL derive the source channel's canonical coverage and place that selection in a new `Masked Areas` channel in the destination, leaving the source unchanged.

Each save, load, duplicate, and drag SHALL record exactly one state in the destination and SHALL NOT threshold or quantize soft coverage.

#### Scenario: Saving with Replace under Selected Areas [chan_convert_save_replace]
- **WHEN** coverage 64 is saved into an existing `Selected Areas` channel with Replace
- **THEN** the channel plane holds 191 for those pixels and loading it returns 64

#### Scenario: Combining under Selected Areas [chan_convert_combine]
- **WHEN** a `Selected Areas` channel holding plane 127 is combined with incoming coverage 64 using Add
- **THEN** the canonical result is 128, the stored plane stays 127, and one state is recorded

#### Scenario: Load applies the sense before Invert [chan_convert_load_invert]
- **WHEN** a `Selected Areas` channel holding plane 64 is loaded with `Invert` checked and New
- **THEN** the effective source is `255 - 64 = 191`, then inverted to 64, and the selection is 64

#### Scenario: A Selected channel loads its sense-aware coverage [chan_convert_load]
- **WHEN** a `Selected Areas` channel whose plane holds 64 is loaded with no invert
- **THEN** the selection coverage is 191 and one `Load Selection` state is recorded

#### Scenario: A duplicate keeps the selection but normalizes the sense [chan_convert_duplicate]
- **WHEN** a `Selected Areas` channel is duplicated within the document or dragged to another document
- **THEN** the destination gains a `Masked Areas` channel holding the same canonical coverage, the source is unchanged, and the destination records one `Duplicate Channel` state

#### Scenario: Soft coverage is not quantized [chan_convert_soft]
- **WHEN** a `Selected Areas` channel holding plane 127 is loaded
- **THEN** the selection coverage is 128 exactly, with no thresholding
