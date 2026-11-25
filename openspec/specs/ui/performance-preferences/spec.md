# ui/performance-preferences Specification

## Purpose
The Performance page of Preferences holds CS6's History & Cache, Scratch
Disks and graphics-processor controls on top of the engine's history,
view pyramid and scratch store (`docs/10-workflow-io/scratch-disks-and-memory.md`).

## Requirements

### Requirement: History States bounds every document's undo states

The Performance page SHALL show a History States spin box from 1 to 1000,
default 20. Its value SHALL bound the number of undo states of every
document. A change SHALL apply at once: each open history SHALL drop its
oldest states while it holds more than the limit and its current state is not
the oldest, and SHALL then drop redo states from the newest end while it
still holds more. Named snapshots SHALL NOT be dropped. The value SHALL be
saved with the preferences and restored at start.

#### Scenario: The default is 20 [pp_history_default]

- **WHEN** the application starts without a saved History States
- **THEN** the spin box shows 20, and a document with 25 edits keeps 20 undo
  states

#### Scenario: Lowering the limit trims open histories at once [pp_history_trim]

- **WHEN** a document has 10 edits and a named snapshot, and History States is
  set to 4
- **THEN** the document keeps 4 undo states and the snapshot, and the History
  panel lists them

#### Scenario: Redo states go when the oldest state is current [pp_history_trim_redo]

- **WHEN** a history of 10 edits is undone to its oldest state and its limit
  is lowered to 3
- **THEN** it keeps its oldest state and the 3 states after it, and the
  current state is unchanged

#### Scenario: History States persists [pp_history_persist]

- **WHEN** History States is set to 50 and the preferences are read back
- **THEN** the saved value is 50 and the engine's limit is 50

### Requirement: Cache Levels caps the view pyramid

The Performance page SHALL show a Cache Levels spin box from 2 to 8, default
8. Its value SHALL cap the number of levels of every view's tile pyramid,
level 0 included. A change SHALL apply at once: a view SHALL rebuild its
pyramid at its next paint. It SHALL be saved and restored like History
States.

#### Scenario: A pyramid keeps at most Cache Levels levels [pp_cache_cap]

- **WHEN** a pyramid is made for a 20000×20000 image with a cap of 3, and with
  a cap of 8
- **THEN** it has 3 levels, and the full count for that size

#### Scenario: A change rebuilds open views [pp_cache_applies]

- **WHEN** a 4096×4096 document is open and Cache Levels is set to 2
- **THEN** the view reports 2 display levels, and 5 after Cache Levels is set
  back to 8

#### Scenario: Cache Levels persists [pp_cache_persist]

- **WHEN** Cache Levels is set to 6 and the preferences are read back
- **THEN** the saved value is 6

### Requirement: History & Cache presets set Cache Levels

The page SHALL offer Tall and Thin, Default and Big and Flat buttons, which
SHALL set Cache Levels to 4, 8 and 8. The page SHALL show Cache Tile Size as a
disabled menu that names the engine's fixed 256 px tile.

#### Scenario: Each preset sets its level [pp_presets]

- **WHEN** Tall and Thin, then Default, then Big and Flat is clicked
- **THEN** Cache Levels reads 4, then 8, then 8, and the engine follows

#### Scenario: Cache Tile Size is fixed [pp_tile_fixed]

- **WHEN** the Performance page is shown
- **THEN** the Cache Tile Size menu is disabled and shows the 256 px tile

### Requirement: Scratch Disks is an ordered list of up to four volumes

The page SHALL list the configured scratch volumes in order, each with its
free space, and SHALL mark a tmpfs volume as using RAM. Add, Remove, Move Up
and Move Down SHALL edit the list. Add SHALL be disabled at four volumes. A
change SHALL apply to the scratch store at once and SHALL be saved. An empty
list SHALL show the default volume as a placeholder, and the first Add to it
SHALL keep the default volume first rather than replace it. Changing the
volumes and reading the scratch readouts SHALL NOT wait for scratch I/O in
progress on another thread: a change made while the store is busy SHALL apply
when it is free, and a readout SHALL show the last reading meanwhile. No
spilled data moves; extents on a removed volume stay readable until freed.

#### Scenario: Volumes are added, reordered and removed [pp_scratch_list]

- **WHEN** two directories are added, the second is moved up, and the first
  listed is removed
- **THEN** the store's volumes and the saved list follow each step

#### Scenario: Adding keeps the default volume [pp_scratch_keeps_default]

- **WHEN** no volume is configured and a directory is added
- **THEN** the list holds the default volume, then the directory

#### Scenario: A busy store does not block the change [pp_scratch_nonblocking]

- **WHEN** another thread holds the scratch store, and the volumes are read
  and then changed
- **THEN** both calls return at once, the reading is the last one, and the
  change applies once the store is released

#### Scenario: Four is the most [pp_scratch_limit]

- **WHEN** four volumes are listed
- **THEN** Add is disabled

#### Scenario: An empty list shows the default [pp_scratch_default]

- **WHEN** no volume is configured
- **THEN** one disabled row names the default volume and Remove is disabled

### Requirement: The page names the graphics processor

The page SHALL show "Detected Graphics Processor:" followed by the adapter's
name next to Use GPU Compute, or "No graphics processor found" when there is
no adapter.

#### Scenario: The adapter is named [pp_gpu_adapter]

- **WHEN** the dialog is given an adapter name, and then an empty one
- **THEN** the line names the adapter, and then reads "No graphics processor
  found"
