## ADDED Requirements

### Requirement: Dense tiles can be packed and spilled without changing their samples

A dense tile SHALL be resident, packed (compressed in RAM), spilled (in a
scratch extent) or unreadable. Every access path of a plane SHALL restore a
packed or spilled tile on first touch, so a reader or writer sees exactly the
samples the tile held before it was evicted, for 8-, 16- and 32-bit planes.
A tile pinned by an access in progress SHALL NOT be evicted.

#### Scenario: Samples are identical after pack, spill and restore [te_identity]

- **WHEN** a seeded 8-, 16- and 32-bit tiled plane has every dense tile packed,
  then spilled, and is read back through tile, row, rectangle and sample access
- **THEN** every sample equals the original bit for bit

#### Scenario: A pinned tile is not evicted [te_pinned]

- **WHEN** an eviction pass runs while a tile is borrowed through `tile_at`
- **THEN** that tile stays resident and the borrow reads the original samples

#### Scenario: A write after a restore is kept [te_write_restore]

- **WHEN** a spilled tile is written through `tiles_mut` and evicted again
- **THEN** reading it back returns the written samples

### Requirement: Eviction frees memory in priority order between thresholds

The governor SHALL evict when the memory in use exceeds 100 % of its ceiling
and stop at 90 %, freeing in order: display tiles (a drop request), tiles only
history holds, then cold live tiles, packing before spilling within each class.
The clock (second-chance) order SHALL skip a recently touched tile once, and a
tile touched under a hot hold SHALL NOT be evicted while the hold or a later
one lives. Above 125 % of the ceiling the allocating thread SHALL also evict.

#### Scenario: History-only tiles go before live ones [te_order]

- **WHEN** a pass must free less than the history-only tiles hold
- **THEN** only bodies with a history hold are packed and the live tiles stay
  resident

#### Scenario: Thresholds start and stop eviction [te_thresholds]

- **WHEN** the pinned host reads 99 %, then 101 %, of the ceiling
- **THEN** nothing is evicted at 99 %, and at 101 % eviction frees until the
  estimate is at or under 90 %

#### Scenario: Second chance and hot holds [te_clock_hot]

- **WHEN** one tile was read since the last pass, and another was read under a
  live hot hold
- **THEN** the first survives one pass and is evicted on the next, and the
  second is not evicted until the hold is dropped

#### Scenario: Urgent pressure evicts on the allocating thread [te_urgent]

- **WHEN** the host reads 130 % of the ceiling and a thread allocates a new
  dense tile
- **THEN** that thread packs cold tiles before the allocation returns

#### Scenario: The display cache is asked to drop first [te_display_drop]

- **WHEN** a pass starts over the ceiling
- **THEN** the display-drop epoch advances before any tile is packed

### Requirement: Clean tiles drop without a write

A tile restored from a scratch extent and not written since SHALL keep that
extent, and evicting it again SHALL write nothing.

#### Scenario: A clean tile is evicted with no I/O [te_clean]

- **WHEN** a spilled tile is read, evicted again, and then written and evicted
- **THEN** the second eviction writes no bytes, and the third writes a new
  extent

### Requirement: A reservation evicts before it is refused

When a reservation exceeds the room and eviction is running, the governor SHALL
evict reclaimable tiles on the calling thread and retry before refusing, so an
edit that fits after eviction is not refused.

#### Scenario: Reclaim turns a refusal into a grant [te_reclaim]

- **WHEN** the room is short of a reservation by less than the cold tiles hold
- **THEN** cold tiles are packed and the reservation is granted

### Requirement: Unreadable tiles fail edits and saves with a typed error

A tile whose scratch read fails SHALL be marked unreadable. An edit that read
an unreadable tile SHALL be rolled back to the history's current state and
report a typed scratch error, and saving a document that holds an unreadable
tile SHALL be refused before anything is written.

#### Scenario: An edit on an unreadable tile changes nothing [te_edit_unreadable]

- **WHEN** a layer's tiles are spilled, the `ReadError` fault is armed, and a
  filter is applied to the layer
- **THEN** the document equals its state before the filter, the history gains
  no state, and the edit reports the scratch error

#### Scenario: Save refuses unreadable tiles [te_save_unreadable]

- **WHEN** a document holding an unreadable tile is saved as PSD
- **THEN** the save fails with the unreadable-tiles error and no file is
  written

### Requirement: Eviction costs nothing when memory suffices

With the memory in use under the ceiling, no tile SHALL be packed or spilled,
and the brush and M-scenario times SHALL stay within 3 % of the previous step.

#### Scenario: No pressure, no eviction [te_idle]

- **WHEN** the pinned host reads 50 % of the ceiling while a document is edited
- **THEN** the packed and spilled counts stay zero

#### Scenario: Benchmarks under MemoryMax [te_memorymax]

- **WHEN** M1–M13 run under `MemoryMax=2G` and `MemoryMax=4G`, and
  unconstrained
- **THEN** no run is OOM-killed or refused (M12 excepted, #1273), the
  unconstrained times are within 3 % of the previous step, and dab p99 under
  pressure is reported against unconstrained
