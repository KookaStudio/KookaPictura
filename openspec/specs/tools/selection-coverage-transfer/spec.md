# tools/selection-coverage-transfer Specification

## Purpose
Moves every cross-document and same-document selection coverage transfer onto one engine owner, so the shell supplies source, destination, and user intent while coverage stays tiled and is never brokered as a canvas-sized dense buffer.

## Requirements

### Requirement: One owner acquires source coverage and applies it

Every selection coverage transfer SHALL be owned by a single transfer mechanism in the engine: Save Selection, Load Selection, Duplicate Channel, channel and selection document drops, and the documents those commands create. The C++ shell SHALL provide only the source document, the source coverage choice (live selection, extra channel, raster mask, or colour row), the destination, and the user's intent (name, Invert, combine operation, fit). No shell adapter SHALL itself export source coverage into a document-sized buffer, reconstruct destination coverage from bytes, or hand source dimensions across the interface.

Acquiring an extra channel or a raster mask SHALL use the existing sense-aware conversions: `alpha:<n>` goes through `selection_from_target` (the channel's `Selected Areas` sense is applied), and `mask:<path>` goes through `selection_from_mask` (a disabled or data-less mask is full coverage). Acquisition SHALL NOT clone a stored channel or mask plane directly.

#### Scenario: Same-document duplicate uses the owner [sct_owner_same_doc]

- **WHEN** Duplicate Channel targets its own source document with an alpha or colour row
- **THEN** the owner acquires that row in place and the destination gains the named copy with exactly one `Duplicate Channel` state

#### Scenario: Cross-document transfers use the owner [sct_owner_cross_doc]

- **WHEN** a channel row is dragged onto another document, a selection outline is dropped on another document, or a dialog targets another same-size document
- **THEN** the owner acquires the source coverage; for New or Replace without Invert the destination gains the same coverage and one destination state, while the source is unchanged. Add, Subtract, and Intersect gain the combined coverage, and Invert flips the source before placement or combination.

#### Scenario: Dialog routes use the owner [sct_owner_dialogs]

- **WHEN** Save Selection targets the current document, another same-size document, or a new document; Load Selection targets the current document or another same-size document; or Duplicate Channel targets the current document, another same-size document, or a new document
- **THEN** the resulting channel, mask, selection, or document matches the behaviour specified for that command

#### Scenario: A disabled mask source is full coverage [sct_mask_disabled]

- **WHEN** a disabled raster layer mask is the source of a Load Selection, and separately when a disabled raster layer mask is the destination of a Save Selection
- **THEN** the source acquires full coverage through `selection_from_mask`, while the destination save keeps its raw stored mask plane as the combine base

### Requirement: Same-document transfers never export a dense plane

When the source and destination are the same document, the owner SHALL acquire the requested coverage in place and SHALL NOT build a document-sized contiguous buffer for the source coverage. This applies to Duplicate Channel's default destination, the Channels panel's New Channel drop when its source is the panel's own document, a Load Selection colour row, and Save Selection into the current document. A destination mask's full-canvas base is destination work, not transport; it is counted by the bound in `No canvas-sized dense transport`.

#### Scenario: Save Selection to the current document exports nothing [sct_save_same_doc]

- **WHEN** Save Selection saves the live selection into a channel or mask of its own document
- **THEN** the owner acquires and applies the coverage in place, builds no document-sized contiguous buffer for the source coverage, and records one `Save Selection` state; a mask destination's full-canvas base is destination work, not transport

#### Scenario: Colour row loads in place [sct_colour_row_same_doc]

- **WHEN** Load Selection loads a colour row of the current document
- **THEN** the row's derived coverage is applied without an intervening document-sized buffer, and one `Load Selection` state is recorded

### Requirement: Coverage stays tiled across a transfer

A transfer whose source coverage is tiled SHALL keep the destination coverage tiled and SHALL share sample storage with the source when the destination output is that coverage itself: a new extra channel, or a same-size load that copies rather than combines. Operations whose result is a distinct plane — resampling to another size, or a boolean combine against existing coverage — SHALL allocate only their necessary output and SHALL still store it tiled.

#### Scenario: A new channel shares the source tiles [sct_tile_share]

- **WHEN** a same-size tiled selection or alpha channel is transferred into a new extra channel
- **THEN** the destination channel shares every source tile and the transfer materializes no plane

#### Scenario: Same-size load shares or combines without a mirror [sct_tile_share_load]

- **WHEN** a same-size tiled selection is loaded into the destination selection with New or Replace and no existing selection
- **THEN** the destination selection is tiled, its bytes equal the source, and no contiguous source mirror is created; Add, Subtract, and Intersect instead produce the combined tiled plane

#### Scenario: Fitted output is necessary allocation [sct_fit_output]

- **WHEN** a tiled source is transferred to a different-sized destination
- **THEN** the fitted result is one tiled destination plane that equals the bilinear fit-and-centre result; `fit_centered`'s dense scratch is the resampler owner's work and is excluded from the transport bound

### Requirement: No canvas-sized dense transport

A coverage transfer SHALL NOT allocate a document-sized contiguous buffer for transport between source and destination. Its heap peak SHALL stay below the necessary destination work plus a numeric transport allowance: at 4250×4250 a tiled source transfer SHALL stay below one 4250×4250 coverage plane plus 1 MiB, and a same-size colour-row transfer to a new channel SHALL stay below one shared derived/retained plane plus 12 MiB (one `TILE`-row band is 4250×256×4 bytes, and a profiled document may add a band-sized `buffer_to_srgb` copy). Necessary destination work is the retained destination output plus the result of a combine or resample: a mask-target save's full-canvas base and written result, an existing-channel combine's result, and `fit_centered`'s dense scratch are destination work, not transport. A transfer that reads tiles without materializing them SHALL still satisfy this bound; a zero materialization count alone SHALL NOT be accepted as proof. The shell adapters SHALL NOT call the whole-selection coverage readers (`selection_coverage_bytes`, `channel_coverage`, `color_row_coverage`) on a transfer path; the readers remain for tests and other reads.

#### Scenario: Tiled source transfer stays below one canvas plane [sct_no_dense_tiled]

- **WHEN** a 4250×4250 tiled selection is transferred to another same-size document and kept as a new channel
- **THEN** the transfer's heap peak stays below one 4250×4250 coverage plane plus 1 MiB

#### Scenario: Colour row retains one plane and transports none [sct_no_dense_colour]

- **WHEN** a colour row is transferred to another same-size document and kept as a new channel
- **THEN** the heap peak stays below one shared derived/retained plane plus 12 MiB, with no transport copy

### Requirement: Preserved transfer semantics

Fit, combine, and history behaviour remains owned by `tools/selection-channels` and `ui/channels-panel`; this capability adds the transport and ownership contract. The transfer owner SHALL preserve the destination rules already in force: the Save and Load dialogs require a same-size destination and treat same-size placement as identity; document drops use the bilinear fit-and-centre placement; Invert is applied before fitting and before combining; Replace, Add, Subtract, and Intersect keep the selection-model algebra; an extra channel's `Selected Areas` sense is applied to derive canonical coverage before Invert, fitting, or combining, and a new destination extra channel is `Masked Areas`; the channel cap, a read-only destination, a missing or invalid target, and an absent live selection refuse without mutation, while the shell's existing open-view checks refuse a closed or documentless source before it reaches the owner.

A successful transfer SHALL record exactly one destination history state: `New` for a new-document transfer and the command label otherwise. A cross-document transfer SHALL NOT record history in the source document; a same-document transfer records the one shared destination state. A refused transfer SHALL record none.

#### Scenario: One state only on success [sct_history]

- **WHEN** any transfer succeeds
- **THEN** the destination gains exactly one state, labelled `New` for a new-document transfer and otherwise for its command; a cross-document transfer leaves the source history unchanged, and a same-document transfer's single state is the shared document's state

#### Scenario: Refusals leave no state [sct_refusal]

- **WHEN** a transfer targets a document at the channel cap, a deleted channel or mask target, or has no live selection or a documentless source
- **THEN** the shell's existing open-view checks keep a closed or documentless source out of the owner, the owner sees only live views, neither document is changed, and no history state is recorded

#### Scenario: A read-only destination refuses [sct_read_only]

- **WHEN** a transfer targets a read-only document
- **THEN** the destination selection, channels, masks, and history are unchanged and the transfer returns false

#### Scenario: Source stays untouched [sct_source_unchanged]

- **WHEN** a selection, channel, or colour row is transferred to a different document
- **THEN** the source selection, channel bytes, and history are unchanged

#### Scenario: Placement policy is unchanged [sct_placement]

- **WHEN** a transfer is same-size, and separately when it targets a different size
- **THEN** a New or Replace transfer without Invert is byte-identical at the same size, a transfer with Invert flips the source first, and a different-size transfer is the fitted, centred bilinear coverage

#### Scenario: A selected channel keeps its canonical coverage [sct_sense]

- **WHEN** Duplicate Channel or a cross-document drag duplicates a `Selected Areas` extra channel, or that channel is loaded as a selection
- **THEN** the source channel's canonical coverage is derived through its sense (`255 - plane`) before Invert and combine, a duplicate is stored in a `Masked Areas` channel, and the source channel is unchanged

### Requirement: New documents come from the same owner

Save Selection's new-image destination and Duplicate Channel's `New` destination SHALL build the document from the transfer owner's acquired coverage, at the source document's dimensions, with the requested name and Invert, recording the same single `New` state as before.

#### Scenario: A new document holds the source coverage [sct_new_document]

- **WHEN** Save Selection or Duplicate Channel targets a new document
- **THEN** a grayscale document opens at the source size with the coverage as its named extra channel and the source is unchanged
