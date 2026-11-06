# document/memory-governor Specification

## Purpose
One process-wide memory governor sets a ceiling from Memory Usage % and the cgroup's limits, tracks memory pressure, and refuses an edit the room under the ceiling cannot hold, so pictura reports "not enough memory" instead of being OOM-killed.

## Requirements

### Requirement: The ceiling respects Memory Usage and the cgroup

One process-wide governor in `pictura-core::memory` SHALL compute a memory
ceiling as the smallest of Memory Usage % × `MemTotal`, the cgroup v2
`memory.max` and `memory.high` of the process's cgroup and of each ancestor
cgroup, over the values that can be read, less headroom of the larger of 5 % of
that limit or 256 MiB. A cgroup value of `max` SHALL mean no limit. A process whose cgroup is the
mount root (`0::/`, a cgroup-namespaced container) SHALL read the root's
limits. When no value
can be read the governor SHALL have no ceiling and SHALL NOT refuse for it.

#### Scenario: A cgroup limit caps the ceiling [mg_cgroup_limit]

- **WHEN** a host has 32 GiB, Memory Usage is 60 % and the process's cgroup has
  `memory.max` 4 GiB, and then `max`
- **THEN** the ceiling is 4 GiB less 256 MiB, and then 60 % of 32 GiB less 5 %
  of it

#### Scenario: An ancestor's limit and memory.high count [mg_nested_cgroup]

- **WHEN** the process sits in a nested cgroup whose own `memory.max` is `max`,
  an ancestor's is 2 GiB, and the leaf's `memory.high` is 3 GiB
- **THEN** the cgroup limit read is 2 GiB, from the ancestor

#### Scenario: A namespaced container reads the root's limit [mg_root_cgroup]

- **WHEN** the process's cgroup line is `0::/` and the cgroup mount's own
  `memory.max` is 1 GiB
- **THEN** the cgroup limit read is 1 GiB and the ceiling 768 MiB

#### Scenario: Headroom is the larger of 5 % or 256 MiB [mg_headroom]

- **WHEN** the limit is 1 GiB, 10 GiB, and 200 MiB
- **THEN** the ceiling is 768 MiB, 9.5 GiB, and zero

#### Scenario: An unreadable host refuses nothing [mg_unreadable_host]

- **WHEN** neither the host's memory nor a cgroup can be read
- **THEN** there is no ceiling and any reservation is granted

### Requirement: Room counts the process's use and what is reserved

The room for a new reservation SHALL be the ceiling less the memory in use and
the bytes already reserved, where the memory in use is the cgroup's
`memory.current` when a cgroup limit is present and the process's resident set
otherwise; and SHALL also be at most the host's available memory less the
larger of 5 % of it or 256 MiB, less the bytes already reserved.

#### Scenario: Use, reservations and available memory shrink the room [mg_room]

- **WHEN** the ceiling is 3 GiB with 1 GiB in use and 512 MiB reserved, and
  then the host has only 1 GiB available
- **THEN** the room is 1.5 GiB, and then 768 MiB less the 512 MiB reserved

### Requirement: The governor keeps a pressure level

The governor SHALL keep a pressure level of Normal, Elevated or Critical from
the `some avg10` value of `/proc/pressure/memory` (Elevated from 5, Critical
from 20) where it can be read, and otherwise from the share of the host's memory
available (Elevated below 20 %, Critical below 10 %).

#### Scenario: PSI sets the level [mg_pressure_psi]

- **WHEN** `some avg10` reads 0.5, 7.25 and 31.0
- **THEN** the level is Normal, Elevated and Critical

#### Scenario: Without PSI the free share sets the level [mg_pressure_fallback]

- **WHEN** `/proc/pressure/memory` is missing and 50 %, 15 % and 5 % of the
  memory is available
- **THEN** the level is Normal, Elevated and Critical

### Requirement: A reservation is a guard that fails with a typed error

`Governor::reserve(bytes, purpose)` SHALL return a `Reservation` that holds its
bytes against the room until it is dropped, or fail with
`MemoryError::Insufficient { purpose, bytes }` when the bytes exceed the room.
The failure SHALL be recorded for the calling thread until cleared. The check
against the room and the addition of the bytes SHALL be one atomic step, so
concurrent reservations never together exceed the room.

#### Scenario: Concurrent reservations never overcommit [mg_reservation_race]

- **WHEN** eight threads each ask for 1 GiB of 1.5 GiB of room at once
- **THEN** exactly one is granted, and nothing stays reserved after it drops

#### Scenario: A reservation is released on drop [mg_reservation]

- **WHEN** a reservation takes 1 GiB of 1.5 GiB of room, a second of 1 GiB is
  asked, and the first is dropped and the second asked again
- **THEN** the second is refused with `Insufficient` naming its purpose and
  bytes, the refusal is recorded, and after the drop it is granted

### Requirement: Reading the host is cheap and injectable

The governor SHALL refresh its reading of the host at most every 250 ms. The
`/proc` and cgroup roots SHALL be injectable, and a test SHALL be able to pin
the reading the calling thread sees.

#### Scenario: A host is read from injected roots [mg_injected_roots]

- **WHEN** a directory holding `meminfo`, `self/status`, `self/cgroup`,
  `pressure/memory` and a nested cgroup tree is read as the host
- **THEN** every field is read from it, and a missing `pressure/memory` leaves
  only PSI unread

#### Scenario: The reading is cached [mg_cached_reading]

- **WHEN** the reading is taken twice within 250 ms while the injected files
  change between the two
- **THEN** the second returns the first reading

### Requirement: Memory Usage is a Performance preference

The Performance page of Preferences SHALL show a Memory Usage spin box from 5 to
100 percent, default 60, whose value the governor uses at once and which is
saved with the session and restored at start.

#### Scenario: The default is 60 % [mg_preference_default]

- **WHEN** the application starts without a saved Memory Usage
- **THEN** the governor's Memory Usage is 60 and the spin box shows 60

#### Scenario: A change applies and persists [mg_preference_change]

- **WHEN** the spin box is set to 45 and the session is saved and read back
- **THEN** the governor's Memory Usage is 45 and the restored session holds 45

### Requirement: Every benchmark path asks the governor before it allocates

New Document, Add Layer, the start of a brush stroke, Undo, Redo, a history
jump, adding a snapshot, restoring a snapshot, Save, and Open SHALL each
estimate their peak bytes and ask the governor before they allocate or change
anything, as the fills, Move, Free Transform and Image Size do. A refused path
SHALL leave the document, its history and its modified flag unchanged, SHALL
record no history state, SHALL report "not enough memory" on the status bar, and
over the control server SHALL answer the error `refused`. Each estimate SHALL be
at least the heap peak the memory benchmark measures for its scenario.

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

### Requirement: History keeps half the ceiling

History's byte cap SHALL be half the governor's ceiling, read when the cap is
enforced; without a ceiling, no state SHALL be dropped for bytes.

#### Scenario: The cap follows the ceiling [mg_history_share]

- **WHEN** the host is pinned to a ceiling of two and a half composites times
  two, and a document's composite is rewritten eight times
- **THEN** history stays within two and a half composites or at two states,
  and still undoes to the previous edit

### Requirement: The control server reports the governor

The control server's `status` SHALL include `memory` with the ceiling, the
memory in use, the bytes reserved, the room (each in bytes, `null` when
unknown), the pressure level, and the Memory Usage percent.

#### Scenario: Status carries the memory object [mg_status_memory]

- **WHEN** `status` is called
- **THEN** `memory` holds `ceiling`, `used`, `reserved`, `room`, `pressure`
  and `usage_percent`, and `usage_percent` follows the preference

### Requirement: No OOM kill under a 4 GiB cgroup

Under `systemd-run MemoryMax=4G` with swap off, the memory benchmark's scenarios
M1–M13 at 17000² SHALL end with no OOM kill: each either completes or is
refused with the memory message.

#### Scenario: M1–M13 under MemoryMax=4G [mg_4g_benchmark]

- **WHEN** `scripts/profile-tools.py --scenarios memory --memory-max 4G` runs
  the catalogue at 17000²
- **THEN** no scenario's outcome is `oom`
