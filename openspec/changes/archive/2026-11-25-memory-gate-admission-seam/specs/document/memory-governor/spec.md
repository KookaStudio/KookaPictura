# Spec Delta

## MODIFIED Requirements

### Requirement: Every benchmark path asks the governor before it allocates

New Document, Add Layer, the start of a brush stroke, Undo, Redo, a history
jump, adding a snapshot, restoring a snapshot, Save, and Open SHALL each
estimate their peak bytes and ask the governor before they allocate or change
anything, as the fills, Move, Free Transform and Image Size do, through the
admission seam of `document/memory-gate-admission`. A refused path SHALL leave
the document, its history and its modified flag unchanged, SHALL record no
history state, SHALL report its refusal on the status bar through the one
presentation path as `document/memory-gate-admission` requires, and over the
control server SHALL answer the error `refused`. Each estimate SHALL be at least
the heap peak the memory benchmark measures for its scenario.

#### Scenario: Each path refuses without room [mg_gates_refuse]

- **WHEN** the room is pinned to zero and New Document, Add Layer, a brush
  stroke, Undo, Redo, adding a snapshot, restoring one, Save and Open are tried,
  then the room is restored
- **THEN** each is refused with the memory reason, the document and history are
  unchanged and no file is written, and with room each runs

#### Scenario: A faked 512 MiB cgroup refuses a fill [mg_fill_under_cgroup]

- **WHEN** the host is pinned to a 512 MiB cgroup with 400 MiB in use and the
  Paint Bucket fills inside a marquee
- **THEN** the fill is refused, no history state is added and the layer's bytes
  are unchanged

#### Scenario: A refusal shows the status message [mg_status_message]

- **WHEN** the room is pinned to zero and Add Layer, Undo and New Document are
  run from the application
- **THEN** each leaves its "not enough memory" message on the status bar, and a
  refused fill still shows the fill message

#### Scenario: The control server answers refused [mg_control_refused]

- **WHEN** the room is pinned to zero and `document new` and `layer_op add` are
  sent
- **THEN** each answers the error `refused` and the document count and layers
  are unchanged

#### Scenario: Each estimate covers its measured peak [mg_estimates_cover_peaks]

- **WHEN** the memory benchmark's scenarios M1, M2, M6, M7, M8 and M11 run at
  4250² alone in their process
- **THEN** each measured heap peak is at most its path's estimate
