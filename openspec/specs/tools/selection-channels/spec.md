# selection-channels Specification

## Purpose
Document-level extra channels that round-trip through PSD and convert to and from a selection.

## Requirements

### Requirement: Document-level extra channels

`pictura-core` MUST expose `Document.channels: Vec<Channel>` for document-level
extra channels (saved selections / spot channels), separate from the per-layer
channel list. A `Channel` MUST carry an `id: i16` and a planar 8-bit `data`
plane whose length equals `width * height` in bytes. A document created with
`Document::new` MUST start with an empty `channels` list.

#### Scenario: New document has no extra channels
- **WHEN** a document is created with `Document::new`
- **THEN** `Document.channels` is empty while available for later saves

#### Scenario: Extra channels are distinct from layer channels
- **WHEN** a document holds both per-layer channels and `Document.channels`
- **THEN** mutating one MUST NOT change the other

### Requirement: Extra channels round-trip through PSD

`pictura-codec` MUST write document-level extra channels into the PSD header
channel count and the image-data section, placing the extra planes after the
color planes. The header channel count MUST equal the color channel count plus
the number of extra channels. On read, the extra planes MUST be restored to
`Document.channels` while `Document.composite` MUST keep only the color planes.
Writing or reading extra channels MUST NOT regress composite or layer handling.

#### Scenario: Single saved selection round-trips
- **WHEN** a document with one extra channel is written and read back
- **THEN** the read document equals the written document and `channels` is restored

#### Scenario: Multiple extra channels round-trip
- **WHEN** a document with two extra channels is written and read back
- **THEN** both channels are restored in order

#### Scenario: Header count includes extra channels
- **WHEN** an RGB document with one extra channel is written
- **THEN** the header channel count is 4 and the composite keeps 3 color planes

#### Scenario: Layer and composite handling is unchanged
- **WHEN** a layered document with no extra channels is written and read back
- **THEN** composite pixels and the layer tree are unchanged

### Requirement: Extra channel validation

`pictura-codec` MUST reject a document whose extra channel data length does not
equal `width * height`, and MUST reject a header channel count that is zero or
exceeds the maximum supported channel count. Invalid input MUST return an error
rather than panicking.

#### Scenario: Length mismatch is rejected
- **WHEN** an extra channel has fewer bytes than `width * height`
- **THEN** `write_psd` returns an error

#### Scenario: Channel count over the cap is rejected
- **WHEN** a file header declares more than the maximum supported channels
- **THEN** `read_psd` returns an error

### Requirement: Selection to and from channel

`pictura-select` MUST provide `Selection::to_channel(id) -> Channel` and
`Selection::from_channel(&Channel, width, height) -> Result<Selection>`. Round-
tripping a selection through `to_channel` then `from_channel` at the same
dimensions MUST reproduce the selection coverage exactly. `from_channel` MUST
return an error when the channel length does not match `width * height`.

#### Scenario: Selection round-trips through a channel
- **WHEN** a selection is converted with `to_channel` and back with `from_channel`
- **THEN** the resulting selection equals the original

#### Scenario: Wrong channel length errors
- **WHEN** `from_channel` is given a channel whose data length is not `width * height`
- **THEN** it returns a size-mismatch error

### Requirement: PSD alpha channel is visible to psd-tools

A PSD written by `pictura-codec` with an extra channel MUST be readable by
`psd-tools`, which reports the bumped header channel count and exposes the extra
plane as the composite alpha channel. This is the independent oracle for the
alternate-channel layout.

#### Scenario: psd-tools reports the written alpha channel
- **WHEN** psd-tools opens a written PSD with one extra channel
- **THEN** it reports color channels plus one, and the alpha bytes equal the extra plane

### Requirement: Document channel budget
A document SHALL hold at most 56 channels in total, counting its colour
channels and its document-level extra channels. The colour-channel count SHALL
come from the recorded source mode when one exists, or the working mode
otherwise; a Multichannel document SHALL use its composite channel count. The
document model SHALL expose a predicate for whether another extra channel may be
added. `New Channel` and
`Save Selection` MUST refuse to add a channel when the budget is full, MUST
leave `Document.channels` and the current selection unchanged, and MUST NOT
record a history state. The PSD codec MUST use the same 56-channel constant.
Saved selections added within the budget SHALL continue to round-trip through
PSD.

#### Scenario: New Channel refuses at the cap [chan_cap_new]
- **WHEN** an RGB document already holds 53 extra channels, so its total is 56, and New Channel is requested
- **THEN** no channel is added and the channel count and history are unchanged

#### Scenario: Save Selection refuses at the cap [chan_cap_save]
- **WHEN** the document is at 56 channels and Save Selection is requested
- **THEN** no channel is added and no history state is recorded

#### Scenario: The budget counts the source mode [chan_cap_source_mode]
- **WHEN** a CMYK document converted to the RGB working mode holds 52 extra channels, so its total is 56
- **THEN** the predicate counts its four source colour channels and refuses another channel; counting only the three working RGB channels would wrongly allow 53 extras and a 57-channel save

#### Scenario: The last free slot is usable [chan_cap_budget]
- **WHEN** the document holds 55 channels
- **THEN** one more channel may be added, and a further attempt at 56 is refused

#### Scenario: The PSD writer shares the budget [chan_cap_write]
- **WHEN** an RGB document with 53 extra channels is written and read back
- **THEN** it succeeds at 56 total, while a document with 54 extra channels, 57 total, is rejected by the writer

### Requirement: Save into an existing extra channel

Saving a selection into an existing extra channel SHALL combine the channel's
current coverage `a` with the incoming selection coverage `b` using the
selection-model algebra and the chosen operation: Replace = `b`;
Add = `max(a, b)`; Subtract = `max(a - b, 0)`; Intersect =
`round(a * b / 255)`. Soft coverage MUST NOT be thresholded or quantized by the
combine. The operation SHALL record exactly one undo state and SHALL make the
previous channel coverage losslessly recoverable through undo.

#### Scenario: Replace overwrites the channel

- **WHEN** a selection is saved into an existing channel with Replace
- **THEN** the channel coverage equals the incoming selection byte for byte

#### Scenario: Boolean operations combine soft coverage

- **WHEN** an existing channel holding 128 is combined with an incoming selection holding 64
- **THEN** Add yields 128, Subtract yields 64, and Intersect yields 32

#### Scenario: Saving into a channel is one state

- **WHEN** Save Selection writes into an existing channel
- **THEN** the history gains exactly one state and undo restores the previous channel bytes

### Requirement: Load with invert and operation

Loading a selection from an extra channel, a raster layer mask, or a colour row
SHALL derive the source coverage, replace it with `255 - coverage` when Invert
is checked, and then combine it with the current selection using the chosen
operation from the selection-model algebra. A colour row SHALL use the
Channels-panel row derivation. For `Select > Load Selection`, every colour
channel counts as visible: the composite row is the luminance of all its colour
channels, and a plate row is the Channels panel's mode-specific derivation for
that row (the colour plane for RGB, the naive ink level for CMYK and
Multichannel, and the luminance for Lab). The Channels panel's own composite row
SHALL continue to follow its eye column. With no current selection, New and Add SHALL
produce the effective source coverage, while Subtract and Intersect SHALL
produce an empty selection. The load SHALL record exactly one undo state.

#### Scenario: Invert is applied before the combine

- **WHEN** coverage 64 is loaded with Invert and Add into a selection of 128
- **THEN** the effective source is 191 and the result is `max(128, 191) = 191`

#### Scenario: No selection with each operation

- **WHEN** a channel is loaded with no current selection
- **THEN** New and Add load the effective source, while Subtract and Intersect load an empty selection

#### Scenario: Load is one state

- **WHEN** a channel is loaded with Add, Subtract, or Intersect
- **THEN** the history gains exactly one "Load Selection" state

#### Scenario: A colour plate loads its plane [sc_load_colour_plane]

- **WHEN** the Red, Green, or Blue row of a flat (100, 150, 200) image is loaded
- **THEN** the source coverage is 100, 150, or 200 per pixel respectively

#### Scenario: The composite row loads its luminance [sc_load_colour_composite]

- **WHEN** the RGB row of a flat (100, 150, 200) image is loaded
- **THEN** the source coverage is the luminance of (100, 150, 200), i.e. 140

#### Scenario: A colour row combines like any other source [sc_load_colour_combine]

- **WHEN** a selection of 200 exists and the Red row holding 100 is loaded with Subtract
- **THEN** the resulting selection coverage is 100 and one "Load Selection" state is recorded

#### Scenario: A derived-mode plate row uses the panel's naive ink [sc_load_naive_mode_rows]

- **WHEN** a CMYK or Multichannel plate row is loaded
- **THEN** the source coverage equals the naive ink derivation the Channels panel uses for that row

### Requirement: Channel names round-trip through PSD

Document extra-channel names SHALL be stored in the PSD alpha-name image
resources: resource 1006 as Pascal strings and resource 1045 as Unicode strings.
On read, 1045 SHALL be preferred and 1006 SHALL be the fallback; names SHALL map
to extra channels by channel order. An extra channel with no stored name SHALL
be displayed as `Alpha N` where N is its one-based position. Writing names SHALL
preserve every other image-resource block and the unparsed tail byte for byte.
Deleting an extra channel SHALL remove its name and shift the following names so
that the remaining channels keep their names. A document whose names are
unchanged SHALL re-emit its original resource bytes.

#### Scenario: ASCII and non-ASCII names round-trip

- **WHEN** extra channels named "Hair" and "Ciel étoilé" are written and read back
- **THEN** both names are restored in channel order

#### Scenario: Missing names fall back

- **WHEN** an extra channel has no stored name
- **THEN** the panel and dialogs display `Alpha N` for its position

#### Scenario: Deleting a channel keeps later names aligned

- **WHEN** the second of three named channels is deleted
- **THEN** the first and third keep their names and the third becomes the second channel

#### Scenario: Unedited names preserve the resource bytes

- **WHEN** a document with alpha-name resources is saved without changing its names
- **THEN** the alpha-name resource bytes are unchanged

#### Scenario: psd-tools reports the names

- **WHEN** psd-tools opens a written PSD with named extra channels
- **THEN** it reports the same alpha-channel names in order

### Requirement: Fit coverage to a different document

`pictura-select` SHALL provide `Selection::fit_centered(target_width,
target_height) -> Selection`. Same-size input SHALL return an identical
selection. Otherwise the method SHALL scale the source plane by
`min(target_width / source_width, target_height / source_height)`, preserve
aspect ratio, centre the fitted plane in the target, and fill the uncovered
border with 0. Scaling SHALL sample the source bilinearly and SHALL NOT
threshold soft coverage. A zero-sized source or target SHALL yield an empty
selection of the requested size.

#### Scenario: Same-size fit is the identity [sc_fit_same_size]

- **WHEN** a selection is fitted to its own dimensions
- **THEN** every byte equals the source selection

#### Scenario: A wide source is letterboxed [sc_fit_letterbox]

- **WHEN** an all-selected 8×4 selection is fitted to 8×8
- **THEN** rows 2–5 are selected and rows 0–1 and 6–7 are empty

#### Scenario: A small source upscales [sc_fit_upscale]

- **WHEN** an all-selected 2×2 selection is fitted to 4×4
- **THEN** the result is all selected

### Requirement: Cross-document selection drag

Dragging a live selection outline with a selection tool from one document to
another document's tab or canvas SHALL copy the selection into the destination.
The source SHALL keep its selection and SHALL NOT record history. The
destination SHALL receive the source coverage fitted with
`Selection::fit_centered` and SHALL record exactly one `Load Selection` state.
A same-size destination SHALL receive the coverage exactly. Dropping on the
source document or outside a document SHALL do nothing. The Move tool and
Ctrl-drag content moves SHALL NOT use this path.

#### Scenario: Same-size drag copies exactly [sc_drag_same_size]

- **WHEN** a selection outline is dragged onto a same-size document
- **THEN** the target selection bytes equal the source selection bytes and the
  target gains one `Load Selection` state

#### Scenario: Different-size drag fits and centres [sc_drag_different_size]

- **WHEN** a selection outline is dragged onto a different-size document
- **THEN** the target selection is the fitted, centred source coverage

#### Scenario: The source keeps its selection [sc_drag_source_unchanged]

- **WHEN** a selection outline is dragged to another document
- **THEN** the source selection bytes and history are unchanged

#### Scenario: A drop on the source is ignored [sc_drag_same_doc_ignored]

- **WHEN** a selection outline is dropped on its own document
- **THEN** the source selection and history are unchanged and the target gains
  nothing

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
