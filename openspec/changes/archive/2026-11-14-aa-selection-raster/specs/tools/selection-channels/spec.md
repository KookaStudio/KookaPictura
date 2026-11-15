# Spec Delta

## ADDED Requirements

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
