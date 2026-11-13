## ADDED Requirements

### Requirement: Opening a PSD admits its tiles through room and scratch

Opening a PSD or PSB SHALL map the file instead of reading it into the heap.
It SHALL bound the decode's contiguous allocations by two thirds of the memory
governor's room. It SHALL bound the layer tiles it decodes by two thirds of the
room plus the free space on the usable scratch disks. A refusal of either kind
SHALL be reported as an open refused for memory. Without a governor reading,
the open SHALL keep the codec's default budget.

#### Scenario: The open budget adds scratch to the tile allowance [oo_budget]

- **WHEN** the open budget is read with 3 GiB of room and 10 GiB of scratch,
  with 3 GiB of room and no scratch, and with no reading
- **THEN** contiguous allocations are bounded at 2 GiB in the first two, tiles
  at 12 GiB and 2 GiB, and the last keeps the default budget unbounded

#### Scenario: A document larger than the room opens when scratch holds it [oo_admits]

- **WHEN** a layered PSD whose layer tiles exceed two thirds of the pinned room
  is opened with ample scratch, and again with no scratch
- **THEN** the first open succeeds and the second is refused for memory

#### Scenario: M12 opens within its bound [oo_m12]

- **WHEN** `scripts/profile-tools.py` runs M12 at 12000² unconstrained, and at
  17000² under `MemoryMax=2G`
- **THEN** the 12000² peak is at most the layer tiles plus 256 MiB, and the
  17000² open completes without an OOM kill or a refusal
