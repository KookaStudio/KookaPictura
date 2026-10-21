# Scratch Disks and Memory

- **Spec ID:** `WF-022`
- **Status:** `Draft`
- **Parity tier:** `Core` — the Performance preferences, scratch disks, History States, cache, GPU toggle, Purge commands, and the Efficiency readout are in CS6 Standard and Extended.
- **New in CS6:** `Changed` — CS6 is 64-bit on macOS and Windows and documents "day-to-day imaging tasks at least 10% faster" than 32-bit; the Mercury Graphics Engine exposes GPU settings in Performance and 3D; the History & Cache section gains **Tall and Thin / Default / Big and Flat** preset buttons (CS5/CS6 Help wording); Auto-recover and Save In Background are new (see `UI-010`).
- **Depends on:** `ARCH-003` performance-targets, `ARCH-009` undo-history, `ARCH-006` gpu-rendering-pipeline, `ARCH-008` document-model, `UI-010` preferences, `11-cross-cutting/crash-recovery-and-autosave.md`, `11-cross-cutting/preference-storage.md`.

> All module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the
> behavioural contract for the feature, and `docs/dev/doc-code-parity-gaps.md`
> records what shipped and what did not. Facts confirmed by the fetched Adobe
> "peak performance" article or the CS6 Help are stated plainly;
> secondary/community numbers are marked *(secondary)*; design choices are
> marked *(inferred)*.

## CS6 behavior

Photoshop keeps working image data, Undo, and History States in RAM and spills
to disk ("scratch") when RAM is insufficient. The **Performance** preferences
panel controls the RAM allocation, scratch disks, History States, cache level
and tile size, and the GPU toggle. The document status bar (and Info panel)
surfaces **Scratch Sizes** and **Efficiency**.

### RAM allocation

- A **Memory Usage** slider/number sets the upper bound of RAM Photoshop may
  use; the article advises adjusting "by no more than 5% at a time", restarting,
  and re-testing. A too-high setting causes "page swapping" with the OS and
  slows Photoshop down.
- 32-bit builds can address ~3 GB; 64-bit builds can address the machine's RAM.
- *Secondary defaults conflict:* Puget Systems reports a default 60 % allocation
  (`ARCH-003`); a community guide reports 70 % (`UI-010`); the fetched Adobe
  article does not state a number. Mark unverified.

### Scratch disks

- Up to **four volumes**; total supported scratch space is stated as **64 EB**.
- Photoshop uses the first-listed drive until it is full, then moves to the
  next. Reorder with the Up/Down keys; place the system/startup drive **last**.
- Prefer a dedicated, fast (SSD or RAID 0) drive that is **not** the same drive
  as the OS virtual-memory/swap file; avoid removable and networked drives.
- Set via `Edit > Preferences > Performance` (Windows) / `Photoshop >
  Preferences > Performance` (macOS).

### Tiles, cache level, cache tile size

- Photoshop splits the image into **tiles** for processing; the default tile
  size is **128 KB**, changeable in the **Cache Tile Size** menu. Larger tiles
  reduce processing time, especially with more than 1 GB RAM.
- **Cache Levels** build a lower-resolution image pyramid for fast screen
  redraws; the Adobe article states a default of **four** levels, and warns
  against setting 1 because the cache is also used by the Healing Brush and
  similar operations. Higher levels speed redraws of large files but use more
  RAM. *(Secondary sources in `UI-010` report a default of 6 and a 1–8 range;
  the fetched Adobe article says 4 — unresolved, see Open questions.)*
- The **History & Cache** section offers **Tall and Thin**, **Default**, and
  **Big and Flat** buttons that choose Level/Tile values based on RAM and CPU
  count.

### History States

- The **History States** preference bounds the undo ring. The Adobe article
  states a **default of 20** and a range of **1–1000**.
- A "full copy at the original size is stored for every operation you perform
  that affects the entire image. Smaller changes, like individual paint strokes,
  require less information per state." More states ⇒ more scratch use.
- Raising the limit increases scratch usage; lowering it frees memory. See
  `ARCH-009` for the tile-diffusion model and retention policy.

### GPU settings

- `Performance > GPU Settings` shows the detected video card and an **Enable
  OpenGL Drawing** option (the Mercury Graphics Engine). A supported card needs
  **≥256 MB VRAM** (≥512 MB for 3D).
- **Advanced Settings** offers `Basic` (least GPU memory; better when sharing
  the GPU), `Normal` (default), and `Advanced` (more features; best for 3D).
  Mode changes take effect only after a restart *(per the article; a CS6 Help
  note calls out a restart for some settings)*.
- `Help > System Info` reports the video card and its VRAM.
- GPU-accelerated features in CS6 include Liquify, Adaptive Wide Angle, Oil
  Paint, Warp/Puppet Warp, the Blur Gallery (requires OpenCL 1.1 per
  `ARCH-003`), Lighting Effects, and 3D.

### Efficiency and Scratch Sizes

- **Scratch Sizes** (status bar / Info panel): the left number is the memory
  currently used by Photoshop to display all open images; the right is the total
  RAM available for processing images.
- **Efficiency:** "the percentage of time actually spent performing an operation
  instead of reading or writing to the scratch disk." 100 % means RAM only;
  values below 100 % mean scratch is being used. The Adobe article treats
  sustained values **below 95 %** during simple operations as a signal to add
  RAM, close other apps, add faster scratch, or reduce History States.

### Purge

- `Edit > Purge` frees RAM held for `Undo`, `Clipboard`, `Histories`,
  `Camera Raw`, or `All`; it **cannot be undone** and is a last resort against
  out-of-RAM errors. `Edit > Purge > Histories` (and `Clear History` in the
  History panel; `Alt`/`Option`-click to purge without changing the image)
  release History states.
- Photos that don't fit in the RAM allocation fall back to scratch; filters are
  documented to run "entirely in RAM" and can raise out-of-memory errors.

### Scratch files on disk

*(secondary)* Photoshop writes temporary files named `Photoshop Temp####` with
a `.tmp` extension into the system temp directory on the chosen scratch volume;
they persist until Photoshop closes cleanly and are a common cause of a
"scratch disk full" state. The location follows the scratch-volume selection.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Edit > Preferences > Performance` | Dialog pane | reported `Ctrl/Cmd+4` | Memory, History & Cache, scratch, GPU. |
| Memory Usage slider + number | Control | — | % of RAM; restart recommended. |
| History & Cache buttons | Buttons | — | Tall and Thin / Default / Big and Flat. |
| History States | Spin/slider | — | Default 20; 1–1000. |
| Cache Levels | Spin/slider | — | Default 4 (Adobe) / 6 *(secondary)*; 2–8. |
| Cache Tile Size | Menu | — | Default 128 KB. |
| Scratch Disks | Checkbox list + order | — | Up to 4 volumes; reorder with arrows. |
| GPU Settings / Enable OpenGL Drawing | Checkbox | — | Detected card shown; restart to apply. |
| GPU Advanced Settings | Sub-dialog | — | Basic / Normal / Advanced. |
| `Help > System Info` | Dialog | — | Video card + VRAM. |
| Status bar / Info panel | Readout | — | Scratch Sizes and Efficiency. |
| `Edit > Purge` | Submenu | — | All / Undo / Clipboard / Histories / Camera Raw. |
| History panel — `Clear History` | Menu | `Alt`/`Option`-click | Purge states without changing pixels. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Memory Usage | % of RAM | 60 *(secondary, unverified)* | ~5–100 | Restart recommended; adjust ≤5% steps. |
| Scratch disks | ordered set | startup disk | 1–4 volumes | First used until full; startup last. |
| Scratch capacity | bytes | — | up to 64 EB total (sourced figure) | Per Adobe article. |
| Cache Levels | int | 4 (Adobe) / 6 *(secondary)* | 2–8 | 1 effectively disables caching. |
| Cache Tile Size | enum | 128 KB | 128 K / 132 K / 1024 K *(secondary)* | Larger = faster, more RAM. |
| History & Cache preset | enum | Default | Tall and Thin / Default / Big and Flat | Sets Level + Tile. |
| History States | int | 20 | 1–1000 | More ⇒ more scratch (`ARCH-009`). |
| GPU drawing | bool | on if card supported | on / off | Requires OpenGL. |
| GPU mode | enum | Normal | Basic / Normal / Advanced | Restart to apply. |
| VRAM minimum | MB | — | 256 (512 for 3D) | Detection gate. |
| Purge scope | enum | — | All / Undo / Clipboard / Histories / Camera Raw | Not undoable. |
| Efficiency floor | % | — | <95 % ⇒ investigate | Adobe guidance, not a hard limit. |

## Algorithms & pipeline

### Memory accounting model

```text
RAM budget        = MemoryUsage% × physical_ram (soft ceiling)
resident working set = tile cache + history store + filter scratch + open docs
efficiency E      = compute_time / (compute_time + scratch_io_time) × 100
if E < 95% (sustained): prefer eviction / spill / suggest RAM or scratch change
```

- The **tile cache** serves the visible tile set without touching scratch on the
  common path; only out-of-window tiles spill (see `ARCH-003`'s cache sizing and
  `ARCH-006`). The budget is shared across all open documents.
- **History** lives in RAM while it fits and spills to scratch (`ARCH-009`); the
  budget is derived from History States and an LRU eviction of the oldest
  non-snapshot tile versions.
- The **soft ceiling** is derived from the RAM allocation and reported; crossing
  it raises a typed error (`ARCH-003`), never a silent OOM.

### Scratch I/O (proposal)

- A single preallocated scratch region per configured volume, grown on demand;
  the first volume is used until full, then the next, matching CS6.
- Prefer a user-chosen fast local filesystem; exclude portal/network paths.
- Treat scratch as page-like blocks addressed by tile id + version; the store is
  content-addressed so undo/redo (`ARCH-009`) and cache (`ARCH-006`) can share
  it.

### GPU selection (proposal)

- Probe the device at startup (VRAM, OpenGL/`wgpu` feature support). If it fails
  the gate, disable `Enable OpenGL Drawing` and all GPU-only features
  (`ARCH-003`, `ARCH-006`); the CPU path must still meet a relaxed budget.
- Basic/Normal/Advanced maps to an internal quality/tile policy, not to a
  different code path.

### Purge semantics

- `Purge` drops reclaimable caches; because it is not undoable it must be routed
  outside the History command bus. Each scope maps to one reclaimable pool:
  Undo/Histories → history store; Clipboard → OS clipboard cache; Camera Raw →
  ACR cache; All → every pool.

### Proposed Linux mapping

| CS6 concept | Linux proposal |
|---|---|
| Scratch disk(s) | Configurable list of directories; default `$XDG_CACHE_HOME/kooka-pictura/scratch/`; user may add any local ext4/xfs/btrfs mount; never network/FUSE if avoidable. |
| Scratch file | Preallocated file (`fallocate`) opened `O_RDWR`; optionally unnamed (`O_TMPFILE`) so it vanishes on crash; `mmap` (e.g. `memmap2`) for zero-copy tile access, with `msync` at checkpoints. |
| Spill when scratch full | Move to next configured directory; if all full, typed `ScratchFull` error and abort the operation without corrupting state. |
| Scratch speed / "fast scratch" | Recommend NVMe/SSD; document RAID 0 as CS6 does. |
| `/tmp` | Do **not** default scratch to `/tmp` (often tmpfs = RAM, no swap); allow it only with a warning. |
| Compressed RAM | `zram` (compressed RAM block device) as a *swap* backing is a natural analogue of "more effective RAM"; a `zram` block device is not a substitute for a real scratch disk because incompressible image tiles can fill RAM. |
| Efficiency | Wall-clock split between compute and scratch read/write; sample with monotonic timers. |
| RAM budget | Soft ceiling = `MemoryUsage% × MemTotal` read from the kernel; respect cgroup limits if present. |
| GPU gate | `wgpu` adapter limits + VRAM query; fall back to CPU when unavailable. |

`zram` details: it creates RAM-based block devices where pages are compressed in
memory; the docs recommend sizing it at no more than about twice RAM because a
~2:1 ratio is expected, and warn that incompressible pages consume full memory.
This is why it is proposed for undo/history-style compressible working sets and
as swap, **not** as the primary store for incompressible image tiles.

## Rust module mapping

Proposals; overlaps `ARCH-003`, `ARCH-006`, `ARCH-009`.

- `pictura-core::memory` — `MemoryPolicy { usage_percent, soft_ceiling,
  reclaimable }`; accounting and `reclaim(scope)` for Purge.
- `pictura-core::scratch` — `ScratchStore` over an ordered `Vec<ScratchVolume>`;
  `alloc(tile_id, version)`, `read`, `write`, `spill`, `flush`, `free`;
  typed `ScratchError::Full | Unavailable | Io`.
- `pictura-core::scratch::file` — preallocate + `mmap` backend; unnamed-file
  mode; `msync` checkpoints.
- `pictura-core::cache::TileCache` — sized from the global budget; LRU
  eviction to scratch.
- `pictura-core::efficiency` — monotonic compute vs scratch-I/O accounting and
  a rolling Efficiency value for the UI.
- `pictura-core::gpu::Probe` — adapter/VRAM gate; `GpuMode` enum
  (Basic/Normal/Advanced).
- `pictura-history::budget` — per-document history cap derived from History
  States, eviction (`ARCH-009`).
- `pictura-core::system` — read `MemTotal`/cgroup limits; enumerate candidate
  scratch volumes (`statvfs` free space, filesystem type).

Crossing types: `Bytes`, `ScratchVolume { path, free, kind }`, `Efficiency`,
`GpuMode`, `PurgeScope`, `MemoryPolicy`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `PerformancePreferences` | `QWidget` | Memory slider, History & Cache buttons, History States, cache level/tile, scratch list, GPU settings. |
| `ScratchDiskListWidget` | `QListView`+buttons | Ordered multi-volume list; up/down reorder; free-space display. |
| `GpuSettingsWidget` | `QGroupBox` | Detected card, Enable OpenGL, Advanced Settings, restart hint. |
| `HistoryCachePresetButtons` | `QButtonGroup` | Tall and Thin / Default / Big and Flat → set Level+Tile. |
| `SystemInfoDialog` | `QDialog` | Video card, VRAM, RAM, scratch info (`Help > System Info`). |
| `StatusBarReadout` | `QStatusBar` widget | Scratch Sizes (used/available) and Efficiency %. |
| `PurgeMenu` | `QMenu` | All / Undo / Clipboard / Histories / Camera Raw. |
| `MemoryEstimateLabel` | `QLabel` | Live estimate as the slider moves. |

Widgets, not QML (`ARCH-003`). The readout is GUI-thread-only and samples
counters published by the Rust core; scratch I/O never runs on the GUI thread.

## Data-model impact

- **Not document data.** Memory policy, scratch volumes, GPU settings, cache
  level/tile, and History States are application/machine preferences; they never
  serialize into PSD/XMP and never enter the History stack.
- **Machine versus user prefs:** per Adobe's file reference, hardware-specific
  values (GPU, cache tile size) live in `MachinePrefs.psp`; the proposal mirrors
  this split with `MachinePrefs` (not roamed) and `PerformancePrefs` (user).
- **Per-document runtime state:** the tile cache, history store, and scratch
  extents are runtime-only and reclaimable at document close.
- **History budget shape:** `HistoryBudget { limit, resident, spilled,
  pinned_snapshots }` per document (`ARCH-009`); Purge/Histories drops the
  unpinned portion.
- **Undo:** changing a Performance preference is not a document History state;
  the dialog's Cancel rolls back uncommitted widget state.

## Edge cases

- **All scratch volumes full/unmounted:** fail the operation with a typed
  `ScratchFull`/`ScratchUnavailable` error and a CS6-style message; never
  corrupt or silently drop states.
- **Scratch volume removed at runtime:** detect I/O errors, mark the volume
  unavailable, spill to the next, and never lose committed tiles.
- **GPU unavailable / device lost:** disable GPU features, fall back to CPU
  within a relaxed budget (`ARCH-003`); no hang.
- **32-bit builds:** not a Linux target (64-bit only); keep a documented ~3 GB
  cap only for parity tests.
- **Huge/PSB documents:** memory grows with the edit, not the canvas, via tile
  diffs (`ARCH-009`); a full-canvas filter legitimately touches all tiles.
- **32-bit float documents:** 4× per-tile memory; eviction and budget must
  account for it.
- **Very large radius blurs/filters:** multi-scale/IIR fallback rather than
  linear-time kernels (`ARCH-003`).
- **Small documents:** per-frame and purge overhead dominates; keep the floor
  well below the frame budget.
- **tmpfs scratch:** warn that it consumes RAM, not disk, and can reproduce
  out-of-memory instead of relieving it.
- **zram with incompressible tiles:** incompressible pages consume full memory;
  do not treat zram as unbounded.
- **Many open documents:** the shared global budget must prevent one document
  from starving others (`ARCH-003`).
- **cgroup-limited containers:** respect the cgroup memory limit rather than the
  host's `MemTotal` when computing the ceiling.
- **SSD wear:** avoid treating scratch as a write-only log; reuse/free extents.
- **Suspend/hibernate:** flush `msync` checkpoints so a crash does not leave a
  partially written scratch file; unnamed files self-clean.
- **Multiple app instances:** namespace scratch directories per instance to
  avoid collisions.

## Parity acceptance criteria

1. Given the Performance pane, Memory Usage, History States (1–1000, default
   20), Cache Levels (2–8), Cache Tile Size (default 128 KB), the scratch list
   (≤4, reorderable), and the GPU toggle are present and persist across restart.
2. Given a document that exceeds the RAM allocation, edits keep Efficiency
   ≥95 % or the shortfall is reported; the app never aborts without a typed
   error.
3. Given sustained Efficiency <95 %, the UI surfaces the value and the
   documented guidance (more RAM / faster scratch / fewer History States).
4. Given `Edit > Purge > Histories` (or `All`), the corresponding memory is
   released and the action is not undoable.
5. Given a full or removed scratch volume, the app spills to the next or fails
   with a clear scratch error; committed document state is intact.
6. Given no GPU or a failed GPU probe, `Enable OpenGL Drawing` is disabled and
   exactly the GPU-dependent features are unavailable; the CPU path still works.
7. Given a GPU mode change (Basic/Normal/Advanced), the restart prompt appears
   and the mode is applied after restart.
8. Given a 32-bit-float document, the memory budget scales with the per-tile
   size and eviction remains correct.
9. Given the status bar readout, Scratch Sizes shows used/available and
   Efficiency shows the compute-vs-scratch percentage.
10. Given a crash while a scratch file is open, restarting leaves no corrupt
    state and the scratch space is reclaimable.

## Sources

- `https://web.archive.org/web/20140204041700/http://blogs.adobe.com/crawlspace/2012/10/how-to-tune-photoshop-cs6-for-peak-performance.html`
  — Adobe (Jeff Tranberry) "How to tune Photoshop CS6 for peak performance".
  Establishes: 32-bit ~3 GB ceiling and 64-bit direct RAM; Memory Usage slider
  and ≤5 % adjustment guidance plus restart; scratch disks (Performance pane,
  reorder, startup drive last, 64 EB across four volumes, prefer fast/SSD/RAID 0
  and avoid removable/networked, separate from OS virtual memory); Efficiency
  definition and the <95 % signal; Scratch Sizes readout; History States default
  20, range 1–1000, and per-operation copy behavior; tile size default 128 KB;
  Cache Levels default four and the warning against level 1; the Tall and
  Thin/Default/Big and Flat buttons; `Edit > Purge`; GPU OpenGL gate, 256 MB
  VRAM (512 MB for 3D), Basic/Normal/Advanced and restart-to-apply; GPU feature
  list.
- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official Photoshop CS6 Help (downloaded with `curl`, text-extracted with
  `pdftotext`). Establishes: `Edit > Purge > Histories` and `Clear History`
  (`Alt`/`Option`-click); History States auto-deletion when the limit is
  reached; Cache Level histogram description and the 2–8 range; the status-bar
  and Info-panel Scratch Sizes/Efficiency definitions; "some filters are
  processed entirely in RAM" and the out-of-memory guidance; Preferences
  overview ("scratch disks"); the 64-bit speed note.
- `https://web.archive.org/web/20240419165453/https://helpx.adobe.com/photoshop/kb/preference-file-names-locations-photoshop.html`
  — Adobe "Preference file functions, names, locations": `MachinePrefs.psp`
  ("a subset of hardware specific preferences like GPU, cache tile sizes") and
  other settings files used for the proposed split. Direct `helpx.adobe.com`
  fetch returns HTTP 403; this is the Wayback capture.
- `https://doc.qt.io/qt-6/qsettings.html` — persistence rules for machine/user
  preference stores.
- `https://doc.qt.io/qt-6/qtemporaryfile.html` — temporary-file and atomic
  rename semantics used by the scratch-file proposal.
- `https://www.kernel.org/doc/html/latest/admin-guide/blockdev/zram.html` —
  Linux zram: compressed RAM block devices, recommended size bound (~2× RAM,
  ~2:1 ratio), incompressible-page caveat; basis for the zram proposal.
- Cross-references: `docs/01-architecture/performance-targets.md` (`ARCH-003`,
  which cites the Puget Systems memory-optimization article and `performance`
  budgets), `docs/01-architecture/undo-history.md` (`ARCH-009`),
  `docs/01-architecture/gpu-rendering-pipeline.md` (`ARCH-006`). The Puget
  Systems articles are cited in `ARCH-003` and were not re-fetched here.

## Open questions

- **CS6 default RAM allocation** and **Cache Levels default.** The Adobe article
  says Cache Levels default 4; `UI-010`'s secondary sources say 6, and Puget
  (`ARCH-003`) reports a 60 % RAM default vs. `UI-010`'s 70 %. *Resolves with:*
  a fresh CS6 Performance pane capture.
- **Cache Tile Size options.** The article says default 128 KB and that the menu
  changes it, but not the full option list (128 K / 132 K / 1024 K is
  secondary). *Resolves with:* a CS6 menu capture.
- **Scratch file naming and directory.** `Photoshop Temp####`/`.tmp` is
  community-sourced; the exact directory (system temp vs. volume root) varies by
  version. *Resolves with:* a CS6 install with an active scratch file.
- **History States default in CS6.** The Adobe article says 20; `ARCH-009`
  notes sources disagree (20 vs 50). *Resolves with:* a CS6 default profile.
- **Restart-required set.** Which Performance fields require a restart in CS6
  (Memory Usage, GPU mode; History/cache?) is not fully documented. *Resolves
  with:* a CS6 toggle-and-observe pass.
- **`MachinePrefs.psp` contents and scope.** The modern reference lists GPU and
  cache tile size; the CS6-era split may differ. *Resolves with:* a CS6 Settings
  folder inspection.
- **zram/mmap strategy validation.** Whether `mmap`-backed scratch plus zram
  meets the `ARCH-003` latency budgets better than plain buffered I/O on the
  reference hardware is untested. *Resolves with:* a scratch-I/O benchmark.
- **cgroup/container behavior.** How to present scratch configuration under
  Flatpak portals (no arbitrary volume access without permission) is open.
  *Resolves with:* `01-architecture/build-and-packaging.md` and
  `11-cross-cutting/security-and-sandboxing.md`.
