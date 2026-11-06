## MODIFIED Requirements

### Requirement: A Move or transform that cannot fit in memory is refused

The Move press, the Move commit and the Free Transform commit SHALL estimate
their peak bytes and SHALL be refused when the estimate exceeds the memory
governor's room (`document/memory-governor`), with the reason reported as not
enough memory. A refused press SHALL start no drag, a refused Move commit SHALL
leave the layers where they were, and a refused transform SHALL be dropped with
the layer restored; none SHALL record a history state.

#### Scenario: Each refuses what memory cannot hold [mmb_gate]

- **WHEN** the governor's room is made smaller than any estimate and a Move
  press, a Move commit after a press that fit, and a flipped Free Transform's
  commit are tried, then the room is restored and the move committed
- **THEN** each is refused with the memory reason, the history and pixels are
  unchanged, the transform session is gone, and the restored move commits

#### Scenario: A transform charges its box and the composite around it [mmb_transform_estimate]

- **WHEN** a 10×10 layer's transform quad is scaled to 20×20, and one with
  fractional corners is estimated
- **THEN** the estimate is four bytes per output pixel plus twelve per pixel of
  the output box joined with the source rect, the fractional box rounded
  outward

### Requirement: History holds a bounded share of memory

After each capture, history SHALL drop its oldest states while the bytes it
holds beyond the live document (retained plane allocations counted once, and
stored tiles) exceed half the memory governor's ceiling
(`document/memory-governor`), and SHALL keep at least the current state and one
undo. Without a ceiling, no state SHALL be dropped for bytes.

#### Scenario: The oldest states go past the cap [mmb_history_cap]

- **WHEN** a document's composite is rewritten eight times under a cap of two
  and a half composites, and five times under a cap that never binds
- **THEN** the first history stays within its cap or at two states, drops
  states, still undoes to the previous edit, and the second keeps every state

#### Scenario: The host's total memory is read [mmb_total_memory]

- **WHEN** `/proc/meminfo` text is parsed and the host is queried
- **THEN** `MemTotal` is read in bytes and the available memory does not
  exceed the total

#### Scenario: Moving and transforming a box on a 17000×17000 document [mmb_large_manual]

- **WHEN** on a host with about 7 GB a 17000×17000 document's painted box is
  moved, resized and transformed repeatedly
- **THEN** pictura is not OOM-killed, and an edit that cannot fit is refused
  with the memory message
