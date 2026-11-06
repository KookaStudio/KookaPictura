# Proposal

## Why

Issue #1211, step 2 of #1209. Two unrelated rules guard memory today: the edit
gate (`helpers/memory_gate.rs`, three quarters of the memory *available*) and
the history cap (`history/budget.rs`, a quarter of *physical* memory). Neither
sees a cgroup limit (Flatpak, containers, `systemd-run MemoryMax=`), where the
kernel kills the app while `MemAvailable` still looks healthy; neither reacts to
pressure; and most of the paths the memory benchmark (#1210) drives are not
gated at all. Under `MemoryMax=4G` every M1–M13 scenario of the baseline
(`docs/dev/benchmarks/memory-baseline.json`) was OOM-killed, M1 (an empty
17000² document) included.

`docs/10-workflow-io/scratch-disks-and-memory.md` (Memory accounting model,
Parity criteria 2) asks for a soft ceiling of Memory Usage % × RAM with cgroup
limits respected, and a typed error, never a silent OOM.

## What Changes

- **One process-wide governor** in `pictura-core::memory`. Its ceiling is
  min(Memory Usage % × `MemTotal`, the cgroup v2 `memory.max` and `memory.high`
  of the process's cgroup and its ancestors) less headroom (the larger of 5 % of
  that limit or 256 MiB). It reads the host at most every 250 ms, keeps a
  pressure level from `/proc/pressure/memory` (or the free-memory share), and
  hands out RAII `Reservation`s that fail with a typed
  `MemoryError::Insufficient { purpose, bytes }`. `/proc` and the cgroup root
  are injectable, so tests fake a host.
- **Fallible canvas-scale buffers.** `try_plane(len, value, purpose)` charges
  the governor for a buffer of 16 MiB or more and allocates it with
  `try_reserve_exact`; an allocator failure is the same typed error, not an
  abort. New Document's planes and the Paint Bucket's match plane use it.
- **Every M1–M13 path asks the governor first** (owner decision: the issue's
  acceptance needs it): New Document, Add Layer, a brush stroke's begin,
  Undo/Redo, adding and restoring a snapshot, Save, Open, and the existing fill,
  Move, Free Transform and Image Size gates. A refusal leaves the document and
  history unchanged, shows "not enough memory" on the status bar, and is an
  error answer (`refused`) over the control server.
- **History's cap is half the governor's ceiling**, replacing a quarter of
  physical memory. The ¾ and ¼ constants go away.
- **A Memory Usage preference**: a 5–100 % spin box on the Performance page,
  default **60 %** (owner decision: `scratch-disks-and-memory.md` over the 70 %
  of `preferences.md`), saved with the session. The full Performance pane
  (slider, RAM readout) stays with #636.
- **The control server's `status` reports the governor** (ceiling, use,
  reservations, room, pressure, Memory Usage %), for the harness.

## Capabilities

### New Capabilities

- `document/memory-governor`
- `document/fallible-allocation`

The issue names them `platform/…`; `openspec/config.yaml` allows no new domain
without a layout change, so they live under `document`, beside
`document/image-resize-memory-budget`.

### Modified Capabilities

- `tools/fill-memory-bounds`: the fill gate reads the governor.
- `tools/move-memory-bounds`: the Move and transform gate reads the governor;
  history's cap is half its ceiling.
- `document/image-resize-memory-budget`: the resize bound is the governor's
  room.

## Impact

`pictura-core` (new `memory` module; the host memory readers move here from
`pictura-codec`, which re-exports them; `libc` and `windows-sys`, already in
`Cargo.lock` and used by `pictura-codec`, become target dependencies of
`pictura-core` for those readers), `pictura-paint` (the flood's buffers),
`pictura-app` (the gates, history's cap, the memory bridge, the preference,
the control server's `status` and refusal answers, the status-bar messages).
