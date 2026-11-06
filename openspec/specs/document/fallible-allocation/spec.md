# document/fallible-allocation Specification

## Purpose
Canvas-scale buffers are allocated fallibly, so an allocation the governor refuses or the allocator cannot satisfy becomes a typed memory error and leaves the edit undone instead of aborting the process.

## Requirements

### Requirement: Canvas-scale buffers are allocated fallibly

`pictura_core::memory::try_plane(len, value, purpose)` SHALL ask the governor
for a buffer of 16 MiB or more before allocating it, and SHALL allocate with
`try_reserve_exact`, so an allocator failure becomes
`MemoryError::Insufficient { purpose, bytes }` recorded for the calling thread,
never an allocation-error abort. A buffer below 16 MiB SHALL NOT be refused by
the governor.

#### Scenario: A plane is built or refused [fa_try_plane]

- **WHEN** planes of 1 KiB and 32 MiB are asked with room, a 32 MiB plane is
  asked without room, a 1 KiB plane is asked without room, and a plane is asked
  after the failpoint is armed
- **THEN** the first two hold their value at their length, the third is refused
  naming its purpose and bytes, the fourth is built, and the failpoint's is
  refused as an allocation failure

### Requirement: A failed allocation leaves the edit undone

New Document's planes and the Paint Bucket's match plane SHALL be built with
`try_plane`; when one fails, the edit SHALL report the memory reason and leave
the document and history as they were.

#### Scenario: A failpoint fails the edit with no change [fa_failpoint_edit]

- **WHEN** the failpoint is armed before New Document and before a Paint Bucket
  click
- **THEN** New Document creates no document, the click adds no history state and
  leaves the layer unchanged, and each reports the memory reason
