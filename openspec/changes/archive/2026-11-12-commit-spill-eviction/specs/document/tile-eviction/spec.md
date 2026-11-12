## MODIFIED Requirements

### Requirement: Eviction frees memory in priority order between thresholds

The governor SHALL evict when the memory in use exceeds 100 % of its ceiling
and stop at 90 %, freeing in order: display tiles (a drop request), tiles only
history holds, cold live tiles, and only then hot tiles, packing before
spilling within each class. A pinned (borrowed) tile SHALL NOT be evicted. The
clock (second-chance) order SHALL skip a recently touched tile once outside the
hot class. A tile SHALL be hot when it was touched while a hot hold was open
and that hold still lives; a closed hold SHALL keep hot only the tiles touched
before it was closed. When the host has a hard memory limit (a cgroup
`memory.max`), the allocating thread SHALL evict above 100 % of the ceiling;
without one, above 125 %. The allocating thread SHALL count the bytes it
allocates since the last host reading and re-read the host once that count
would cross the threshold.

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
  live hot hold, and passes need no more than the cold tiles free
- **THEN** the first survives one pass and is evicted on the next, and the
  second is not evicted until the hold is dropped

#### Scenario: Hot tiles go last [te_hot_last]

- **WHEN** a pass needs more than every cold tile frees, and some tiles are hot
- **THEN** it evicts the hot tiles after the cold ones, and a pinned tile stays
  resident

#### Scenario: A closed hold keeps only what it saw [te_hold_closed]

- **WHEN** a tile is read under a hold, the hold is closed but kept, and then
  another tile is read
- **THEN** a pass that needs no more than the cold tiles free leaves the first
  resident and evicts the second

#### Scenario: Urgent pressure evicts on the allocating thread [te_urgent]

- **WHEN** the host reads 130 % of the ceiling and a thread allocates a new
  dense tile
- **THEN** that thread packs cold tiles before the allocation returns

#### Scenario: A hard limit evicts on the allocating thread at the ceiling [te_urgent_hard]

- **WHEN** a host with a cgroup `memory.max` reads 101 % of the ceiling and a
  thread allocates a new dense tile, and, separately, a host with only a RAM
  total reads 110 % of its ceiling
- **THEN** the first allocating thread evicts cold tiles, and the second does
  not

#### Scenario: An allocation burst is caught between passes [te_alloc_estimate]

- **WHEN** the last host reading was 95 % of a hard-limited ceiling, the host
  now reads 101 %, and a thread allocates more than the remaining 5 % in new
  dense tiles before the background pass runs
- **THEN** that thread re-reads the host and evicts cold tiles

#### Scenario: The display cache is asked to drop first [te_display_drop]

- **WHEN** a pass starts over the ceiling
- **THEN** the display-drop epoch advances before any tile is packed

## ADDED Requirements

### Requirement: The view keeps only the tiles it built hot

The view SHALL hold open a hot hold only while it builds its display tiles,
then close it and keep it until the presented view changes. A tile that
another operation reads or writes afterwards SHALL NOT be hot because of the
view.

#### Scenario: A commit's tiles are not the view's [te_view_hold]

- **WHEN** the presented view's tiles are built, and then another plane's tiles
  are read while the view is unchanged
- **THEN** that plane's tiles are not hot, and the layer tiles the view read
  stay hot until the presented view changes

### Requirement: A large commit under a hard limit spills instead of being killed

A Free Transform commit whose output does not fit the memory under a hard
limit, but does fit that memory plus the free scratch disk, SHALL complete by
packing and spilling tiles and SHALL NOT be killed for memory.

#### Scenario: A 17000² commit under MemoryMax=2G spills [te_cgroup_commit]

- **WHEN** a 17000 × 17000 document with a full-layer gradient on Layer 1,
  under `systemd-run --user -p MemoryMax=2G` with several GB of free scratch,
  commits Free Transform at ×1.5, and in a fresh run at ×1.0 with 15°
- **THEN** each commit lands one "Free Transform" history state, the process
  is not killed, and the scratch disk holds spilled tiles
