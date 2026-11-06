# tools/fill-memory-bounds

## MODIFIED Requirements

### Requirement: A fill that cannot fit in memory is refused

Before it allocates, the Gradient, the Paint Bucket, the Fill and the Stroke
SHALL estimate their peak bytes (their box-sized planes, the bands they fork at
the target layer's grown rect — the union of its current rect and the fill's box
— and the display refresh of the box) and SHALL refuse the edit, recording no
history state, when the estimate exceeds the memory governor's room
(`document/memory-governor`). The refusal SHALL be reported as "not enough
memory" on the status bar, and the next fill that fits SHALL run. When the host's
memory cannot be read, the gate SHALL NOT refuse.

#### Scenario: Each fill refuses what memory cannot hold [fmb_gate]

- **WHEN** the governor's room is made smaller than any fill's estimate and a
  bucket click, a gradient, a Fill and a Stroke are tried, then the room is
  restored and the Fill tried again
- **THEN** each is refused with the memory reason, the pixels and history are
  unchanged, and the restored Fill commits one state and clears the reason

#### Scenario: A bucket in a marquee on a 17000×17000 document [fmb_large_manual]

- **WHEN** a 17000×17000 document is opened on a host with about 7 GB, a
  rectangular marquee is drawn and the Paint Bucket clicks inside it, then a
  fill is tried with too little memory free
- **THEN** the fill completes without the process being killed, and a fill that
  cannot fit is refused with the memory message instead of crashing

#### Scenario: A fill that grows an empty layer is charged for the grown bands [fmb_grown_bands]

- **WHEN** a fill's box lies outside a small layer's rect
- **THEN** its estimate covers the bands of the union rect, not only the old
  layer's bands
