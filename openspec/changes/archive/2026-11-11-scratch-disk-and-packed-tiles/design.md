# Design

## Decisions taken with the owner (2026-11-11)

- **R7 compressor.** Benchmark `lz4_flex` against `flate2` on M12 tiles and keep
  the faster one if it meets the targets (unconstrained within 3 % of step 8;
  dab p99 ≤ 2× unconstrained). The numbers are in §Benchmarks.
- **R8 scratch I/O.** `pread`/`pwrite` (`std::os::unix::fs::FileExt`), not
  `mmap`. Page-cache pages count against a cgroup either way, but written pages
  can be reclaimed, and a failed read is an `Err` rather than a `SIGBUS`. The
  benchmark against `libc::mmap` settles the spec's open question.
  `scratch-disks-and-memory.md` is updated in a separate `TASK-ALLOWS-DOCS`
  commit.
- **M12 under 4G** stays with #1273 (the PSD decode peak) and is reported as
  blocked there.
- **One PR**, closing #943. Scratch volumes are a saved session setting with
  no UI; #636 adds the Performance pane.

## 1. Tile bodies

`Tile::Dense(Arc<[T]>)` becomes `Tile::Dense(Arc<Body<T>>)`.
`tiled_plane/body.rs` defines a body as:

```text
Body<T> {
    len,                 // samples
    state: AtomicU8,     // RESIDENT | EVICTING | OTHER
    pins: AtomicU32,     // live borrows (TileRef, TileMut, copies)
    referenced: AtomicBool,  // clock bit
    hot: AtomicU64,      // epoch of the last access under a hot hold
    history: AtomicU32,  // history holds (§4)
    cell: Mutex<Cell<T>>,
    data: UnsafeCell<*const T>   // valid while RESIDENT and pinned
}
Cell<T> { slot: Resident(Box<[T]>) | Packed(Box<[u8]>) | Spilled | Unreadable,
          extent: Option<Extent> }   // the clean copy on disk (R3)
```

Sharing and copy-on-write are unchanged: a plane clone or a history delta clones
the `Arc`. A write forks a body held by more than one plane. The census keys on
the body's address, which stays stable while the body is alive.

**Pins.** A reader increments `pins` (SeqCst) and then loads `state`
(SeqCst). If the state is `RESIDENT`, the reader borrows the samples without a
lock. Otherwise it locks the cell, restores the samples, and sets `RESIDENT`.
The evictor locks the cell, stores `EVICTING` (SeqCst), and loads `pins`
(SeqCst). If any pin is held, it stores `RESIDENT` back and skips the body.
Under SeqCst order, either the reader sees `EVICTING` and takes the lock, or the
evictor sees the pin. So a pinned body is never moved, and the evictor reads
samples only while it holds the lock with no pins. `TileRef`, `TileMut`, and
every internal copy hold a pin guard that unpins on drop. `TileData::Dense`
carries the guard, so it is no longer `Copy`, and matches on it take a
reference. The public fields and methods are unchanged.

**Restore on touch** (R1). The first pin on a body that is not resident
restores it on that thread:

- *Packed*: decompress.
- *Spilled*: `pread` the extent, verify its CRC-32, then decompress. The extent
  is kept (clean).
- *Unreadable*: there is nothing to restore (§5).

Restore time counts as scratch I/O for Efficiency.

**Writes.** Callers move `TileMut::data` out of the `TileMut` (the stroke
raster and the uniform fill), so a writer's pin cannot live in the `TileMut`.
Instead, `tiles_mut` pins each body and records its index in the plane. The
plane releases those pins at its next mutable call or when it drops. The
borrow checker guarantees by then that no `&mut [T]` from the earlier borrow is
alive. A plane that is never mutated again only keeps those tiles from being
evicted, which is never unsafe. A write drops the body's clean extent, so a
tile that is written is no longer clean.

`span_in_tile` used to borrow a tile's slice past the call. It now copies, as
`row_span` already does for a span that crosses tiles.

## 2. The scratch file (`pictura-core::scratch`)

- **Volumes.** An ordered list of up to four directories, set with
  `scratch::set_volumes`. The default is `$XDG_CACHE_HOME/kooka-pictura/scratch/`
  (or `~/.cache/…`). `statfs` flags a `tmpfs` volume (`TMPFS_MAGIC`), and the
  readout and the app's log carry a warning for it. Volumes under `/tmp` are
  accepted but flagged the same way.
- **The file.** Each volume opens on first spill with `O_TMPFILE | O_RDWR`. Where
  the filesystem refuses that flag, the store creates
  `pictura-scratch-<pid>-<n>` and unlinks it at once. A crash therefore leaves
  nothing behind (Parity criterion 10). On start, the store also deletes files
  of that name whose PID is not alive.
- **Extents.** Sizes are 4 KiB-aligned. A first-fit free map keyed by offset
  coalesces neighbours. When nothing fits, the file grows by
  `fallocate(max(need, 64 MiB))`. `ENOSPC` (or the injected full disk) marks the
  volume full, and the store moves to the next one. When every volume is full,
  `spill` returns `ScratchError::Full`, and the evictor pauses spilling with
  backoff from 100 ms up to 5 s (Parity criterion 5). A freed extent goes back
  to its volume's map, and a full volume becomes usable again once an extent on
  it is freed.
- **Checksums.** Each extent records CRC-32 (`crc32fast`), its length, and its
  volume. A read that fails or a checksum that does not match returns
  `ScratchError::Unreadable`. `crc32fast` is already in the lock file (through
  `flate2`); it becomes a direct dependency of core.
- **I/O.** `write_all_at` and `read_exact_at`, with no page-cache advice. The
  first version called `posix_fadvise(DONTNEED)` after each read: on pages
  still dirty that forces a synchronous writeback, and reads fell to 19 MiB/s.
  The page cache is already excluded from `used` (§6).
- **Failure injection.** `scratch::inject(Fault::{Full, ReadError, Corrupt,
  RemoveVolume(i)})`. Each fault is armed until cleared, and is process-wide.
  The Rust tests and the Qt suites (through the bridge) use it.
- **Scratch Sizes.** `scratch::sizes()` returns `{ used, available }`. `used`
  is the bytes of live extents, and `available` is the sum of `statvfs`
  available space over the usable volumes plus the free extents inside the
  files. Per-volume rows give the path, file size, used bytes, free space, and
  the full and tmpfs flags.
- **Efficiency.** Cumulative monotonic nanoseconds `{ compute, io }`, as
  `scratch::efficiency()`. `io` is the time threads spend blocked on a scratch
  read, a write, or a synchronous reclaim; the background evictor's I/O blocks
  nobody, so it is not counted. `compute` is the wall time of operations run
  under `scratch::operation()` (the app wraps each edit's commit window and
  each worker job), less the `io` inside them. Efficiency is
  `compute / (compute + io)` over the change between two readings, and 100 %
  when nothing was measured.

## 3. Eviction (`pictura-core::memory::eviction`)

- **Registry.** Every new body registers a `Weak<dyn Evictable>` in one of 16
  sharded lists, chosen by thread. A clock hand walks the shards in turn, and
  dead entries are removed during the walk. `Evictable` is the type-erased view
  of `Body<T>` for `u8`, `u16`, and `f32`.
- **Thread.** `eviction::start()` spawns one thread, from the app at start-up
  and from tests explicitly. Every 100 ms, or when an allocation wakes it, the
  thread reads the host without the 250 ms cache. When `used > ceiling`, it
  frees down to `0.9 × ceiling`. In the cgroup case, `used` is
  `memory.current − file` (§6).
- **Order** (R2):
  1. Raise the display-drop epoch. The app's next present drops its unpinned
     display tiles, the same as at Critical pressure.
  2. *Backed* tiles (#1218) have a slot in this order but no implementation
     here.
  3. Bodies with a history hold: pack them, then spill the packed ones.
  4. All other bodies (cold live tiles): pack, then spill.

  In each pass the clock skips a body that is pinned or hot. A body whose
  `referenced` bit is set has the bit cleared and is skipped (second chance).
  Packing keeps the compressed bytes only when they are smaller than 7/8 of the
  raw bytes; an incompressible body is spilled raw instead. A body with a clean
  extent is dropped straight to *Spilled*, with no write (R3). After each pass,
  the thread calls `malloc_trim(0)` (glibc), because freed 64 KiB tiles stay in
  the heap otherwise and RSS would not fall.
- **Urgent.** When `used > 1.25 × ceiling`, the thread sets `URGENT`. A thread
  that allocates a new dense body then runs one reclaim of that body's size
  before it allocates.
- **Reclaim before refusal.** `Governor::reserve` calls
  `eviction::reclaim(shortfall)` on the calling thread when the room is short,
  and only while eviction is running. It then reads the host again and retries
  once, and refuses only after that.
- **Hot holds.** `eviction::hot()` returns an RAII hold. Each new hold takes a
  new epoch, and a body touched while any hold is alive records that epoch. A
  body is hot while its epoch is at or after the oldest live hold's. The app
  holds one across each paint stroke (begin to end) and one per visible-view
  build, renewed whenever the visible tile set changes. Tiles read while the
  current view was composited therefore stay resident. *Visible* is
  approximated by "read since the view last changed": a cached display tile
  does not read its layer tiles again.

## 4. History holds

`Plane::history_tile(i)` returns a `SharedTile` that counts a history hold on
its body; dropping or cloning the `SharedTile` adjusts the count, and
`swap_tile` moves the hold to the body it now holds. History's `TileDelta`
uses this, so the evictor sees the bodies that only history keeps. A delta
holds the old side of a changed tile, which by construction is no longer live.

## 5. Unreadable tiles

A body whose restore fails becomes *Unreadable*. Its extent is freed, it reads
as `T::default()`, and every read increments `scratch::unreadable_reads()`.
`Plane::unreadable_tiles()` counts such bodies, and is constant-time when none
exist anywhere.

- **Edits** (app). `apply_edit_with` compares `unreadable_reads()` with the
  value it recorded after the last commit. When the count moved and the
  history's current state holds an unreadable tile, the edit read data it does
  not have: the document is restored to the current state (no state change),
  `edit_refused_for_scratch()` becomes true, and the frame reports "The edit
  needs image data that could not be read back from the scratch disk." The
  attribution is approximate: a display rebuild of the same document in the same
  window also counts. This errs toward refusing, and is the safe side.
- **Save** refuses a document whose planes hold an unreadable tile, with a typed
  `UnreadableTiles` error, before anything is written.

## 6. Governor changes

`HostReading` gains `cgroup_file`, the `file` line of `memory.stat`. In the
cgroup case, `used_of` becomes `memory.current − file`: written scratch pages
sit in the page cache, which the kernel reclaims before an OOM kill, so
counting them would make the evictor spin. `Governor::reading_fresh()` bypasses
the 250 ms cache for the evictor.

## Benchmarks

All numbers are from the reference machine (16 CPUs, 29 GiB, NVMe), on
release builds.

**R7: the packing codec.** These are the 11,317 dense tiles (705 MiB) of the
12000² M12 fixture (`pictura-codec` `pack_codec_benchmark`):

| Codec | Ratio | Pack | Unpack |
|---|---|---|---|
| `lz4_flex` 0.11 (block) | 0.016 | 7461 MiB/s | 2465 MiB/s |
| `flate2` (`Compression::fast`) | 0.014 | 1666 MiB/s | 2708 MiB/s |

LZ4 packs 4.5× faster at nearly the same ratio and unpacks at about the same
speed. Packing is what the evictor and an urgent allocating thread pay, so
`lz4_flex` is kept: a pure-Rust MIT crate that `deny.toml` already allows.

**R8: scratch I/O.** These are 4096 extents of 64 KiB (256 MiB) on the default
volume, warm page cache, with the CRC-32 check included in reads
(`pictura-core` `scratch_io_benchmark`):

| Access | Write | Read |
|---|---|---|
| `pread`/`pwrite` | 1231 MiB/s | 6669 MiB/s |
| `mmap` (`MAP_SHARED`) | 882 MiB/s | 7028 MiB/s |

Reads are equal within noise, and writes favour `pwrite`. Since `pread` turns
an I/O error into an `Err` rather than a `SIGBUS`, the spec's "zram/mmap
strategy validation" question is settled in favour of `pread`/`pwrite`.

## Risks

- **The cost of an access path with no pressure.** Each tile access now costs a
  pin and a state load, and a new dense tile costs a second allocation and a
  registry push. These are measured against the 3 % budget. A per-sample
  `Plane::sample` loop pays one pin per sample, and is the first suspect if the
  budget is missed.
- **RSS lag.** Freeing memory lowers the reading only after `malloc_trim`, and
  the cgroup counter reflects the drop only at the next 100 ms tick. The 90 %
  stop target absorbs the lag.

**Unconstrained cost.** This compares `profile-tools.py` default scenarios
(4000×3000, 71 scenarios over 5 ms) on master `b017d378` and this branch, built
in separate worktrees and run alternately three times each, using the median
`total_ms` per scenario:

- **Overall.** The geometric mean is **1.026**, within the 3 % budget.
- **Brushes.** Unchanged: brush-40 0.95×, brush-500 0.99×, brush-1940 1.07×.
- **Shape commits.** Median about 10 % slower (custom 159 → 191 ms, rounded
  rectangle 166 → 186 ms, polygon 358 → 397 ms). Their run ranges overlap
  master's, so this is a shift to watch rather than a measured regression.
- **`type-warp-preview`.** Bimodal on both builds (about 125 or 265 ms), so
  it is noise.

**Under `MemoryMax`** (`profile-tools.py --peak --memory-max`):

- **M1–M11 and M13, at 17000² under `MemoryMax=2G`.** Every scenario completes,
  with no OOM kill and no refusal; the largest peak is 597 MiB (M8). Steps 1–8
  already keep these scenarios small, so they barely exercise eviction. The
  eviction paths are covered by the `memory::eviction` tests.
- **M12, at 12000² under `MemoryMax=4G`.** Refused at the open: the PSD decode
  is gated as a whole (#1273), which is out of scope here. The 8-dense-layer
  variant waits on the same fix.
- **Dab p50/p99 under pressure.** Not measured: no M scenario reaches the
  ceiling once steps 1–8 apply. This is a manual row in the test plan.
