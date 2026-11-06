# tools/move-memory-bounds Specification

## Purpose
Keeps a Move and a Free Transform on a large document at the size of what they
move, refuses one the host cannot hold, and bounds the memory history keeps.

## Requirements

### Requirement: A Move works at the size of what it moves

The Move tool's moving image for a single pixel layer SHALL cover only the
layer's visible pixels and SHALL read the layer's planes without materializing
a banded plane whole. A cached preview SHALL be placed where the moved layer's
pixels are. The commit SHALL damage, for a pixel layer without effects that is
not a group or an adjustment, only its visible pixels before and after the
move.

#### Scenario: A bounded read is the crop of the whole [mmb_bounded_read]

- **WHEN** a canvas-sized layer with a patterned opaque mark is read whole and
  over the mark's box, and a layer whose banded planes have diverged from
  history is read over the box
- **THEN** the box read equals the crop of the whole read, a box outside the
  layer reads nothing, and no plane is left with a materialized copy

#### Scenario: A painted mark moves at its own size [mmb_mark_move]

- **WHEN** a 40×40 mark is filled on a new canvas-sized layer of an 800×600
  document, moved by (30, 20), and pressed again
- **THEN** the moving image is 40×40 at the mark, the commit's damage is at
  most 80×80, the mark shows at its new place, and the cached preview sits at
  the moved mark

#### Scenario: The damage is the mark before and after [mmb_damage]

- **WHEN** a canvas-sized layer with a mark is translated, and a fully opaque
  layer is translated
- **THEN** the first damage is the mark's box before and after, and the second
  is the layer's rect before and after

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
