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

Loading a selection from an extra channel or raster layer mask SHALL derive the
source coverage, replace it with `255 - coverage` when Invert is checked, and
then combine it with the current selection using the chosen operation from the
selection-model algebra. With no current selection, New and Add SHALL produce
the effective source coverage, while Subtract and Intersect SHALL produce an
empty selection. The load SHALL record exactly one undo state.

#### Scenario: Invert is applied before the combine

- **WHEN** coverage 64 is loaded with Invert and Add into a selection of 128
- **THEN** the effective source is 191 and the result is `max(128, 191) = 191`

#### Scenario: No selection with each operation

- **WHEN** a channel is loaded with no current selection
- **THEN** New and Add load the effective source, while Subtract and Intersect load an empty selection

#### Scenario: Load is one state

- **WHEN** a channel is loaded with Add, Subtract, or Intersect
- **THEN** the history gains exactly one "Load Selection" state

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
