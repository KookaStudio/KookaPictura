## ADDED Requirements

### Requirement: Spilled tiles round-trip through a checksummed scratch file

`pictura-core::scratch` SHALL store a byte payload in an extent of a scratch
file and SHALL read back exactly those bytes. Each extent SHALL record a
CRC-32 checksum of its payload, and a read whose checksum does not match, or
whose I/O fails, SHALL return a typed `Unreadable` error instead of data.
Freed extents SHALL be reused by later writes.

#### Scenario: A payload reads back unchanged [sd_round_trip]

- **WHEN** seeded payloads of sizes from 1 byte to 300 KiB are written to a
  scratch store on a temporary directory and read back
- **THEN** every read returns the bytes written

#### Scenario: A corrupted extent is unreadable [sd_corrupt]

- **WHEN** the `Corrupt` fault is armed while a payload is written, or the
  `ReadError` fault is armed while it is read
- **THEN** the read returns `ScratchError::Unreadable` and no data

#### Scenario: Freed extents are reused [sd_free_reuse]

- **WHEN** a store writes ten payloads, frees them all, and writes ten payloads
  of the same sizes
- **THEN** the scratch file does not grow and used bytes return to the
  payloads' total

### Requirement: Volumes fill in order and a full scratch is a typed error

The store SHALL use an ordered list of at most four volumes, defaulting to
`$XDG_CACHE_HOME/kooka-pictura/scratch/`. When a volume is full or removed, the
store SHALL write to the next one; when every volume is full it SHALL return
`ScratchError::Full` and leave existing extents intact. A volume on `tmpfs`
SHALL be reported with a warning flag.

#### Scenario: A full volume falls through to the next [sd_fallthrough]

- **WHEN** two volumes are configured and the first is made full by fault
  injection
- **THEN** the next write lands on the second volume and reads back unchanged

#### Scenario: All volumes full returns ScratchFull [sd_all_full]

- **WHEN** every volume is full
- **THEN** a write returns `ScratchError::Full` and earlier extents still read
  back

#### Scenario: A removed volume falls through [sd_removed_volume]

- **WHEN** the `RemoveVolume(0)` fault is armed with two volumes configured
- **THEN** writes land on the second volume and reads of extents on the first
  return `Unreadable`

#### Scenario: The default volume and tmpfs warning [sd_default_volume]

- **WHEN** no volume is configured, and then a volume on a `tmpfs` mount is
  configured
- **THEN** the default is `$XDG_CACHE_HOME/kooka-pictura/scratch/`, and the
  tmpfs volume's row carries the warning flag

### Requirement: The scratch file leaves nothing behind

The scratch file SHALL be created unnamed (`O_TMPFILE`), or created and
unlinked at once where the filesystem refuses that flag, so a process killed
with a scratch file open leaves no file on any volume.

#### Scenario: SIGKILL leaves no scratch file [sd_crash_clean]

- **WHEN** a child process spills payloads to a scratch volume and is killed
  with SIGKILL, and a new store then opens the same volume
- **THEN** the volume directory holds no scratch file and the new store
  writes and reads normally

### Requirement: The engine reports Scratch Sizes and Efficiency

The engine SHALL report Scratch Sizes as the bytes held in scratch extents and
the bytes available across the usable volumes, and Efficiency as monotonic
cumulative compute and scratch-I/O times whose ratio over an interval is
`compute / (compute + io) × 100`, or 100 % when nothing was measured. The
control server's `status` and the bridge SHALL carry the volume list, Scratch
Sizes, Efficiency and the packed, spilled and unreadable tile counts.

#### Scenario: Efficiency splits compute from I/O [sd_efficiency]

- **WHEN** an operation runs 30 ms of work that includes 10 ms of scratch I/O,
  and the readout is taken before and after
- **THEN** both counters only grow, compute grows by about 20 ms and I/O by
  about 10 ms, and the interval's Efficiency is about 67 %

#### Scenario: Scratch Sizes follow the extents [sd_sizes]

- **WHEN** payloads are written and then freed
- **THEN** used grows by their aligned sizes and returns to zero

#### Scenario: status carries the scratch readouts [sd_status]

- **WHEN** a client asks the control server for `status`
- **THEN** the reply's `memory` object carries `scratch.volumes`,
  `scratch.used`, `scratch.available`, `efficiency.percent` and the tile counts
