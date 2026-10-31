# Design

## Context

See `proposal.md` for the crash and its evidence. The constraints that shape
the implementation:

- Two independent device-creation sites exist: `request_device()` in
  `crates/pictura-render/src/gpu/backend.rs:237-299` and `create_device()` in
  `crates/pictura-app/src/gpu.rs:58-93`. Both hardcode Vulkan.
- `wgpu::Instance::new` initializes every backend in its bitmask
  (`wgpu-core-30.0.1/src/instance.rs:83-135`), so a multi-backend instance
  loads the Vulkan ICD and crashes before adapter selection.
- `devices()` and `gpu_composite_verified()` are process-global `OnceLock`s
  (`backend.rs:319-328`, `gpu/mod.rs:312-316`). A process cannot change
  backend after the first device.
- The first GPU touch in a GUI launch is the frame constructor probe
  (`crates/pictura-app/cpp/frame.cpp:151-155`), after `main()` and after the
  state-home setup.
- `PICTURA_VERSION` is a public compile definition on `pictura_shell`
  (`CMakeLists.txt:899-902`), so every app source and every Qt Test sees it.
- `crates/pictura-render/src/gpu/backend.rs` is 1150 lines (cap 1200) and
  `frame.cpp` is 1101 (cap 1200).

## Goals / Non-Goals

**Goals:**

- One pinned backend per process, chosen before the first device.
- A driver crash or hang during probing costs one child process, not the app.
- A crash during the parent's own init costs one launch, not a crash loop.
- Per-backend outcomes survive a restart and are invalidated by app version.
- Linux rendering output and `gpu-parity` are byte-identical to today; the
  status label becomes `GPU (Vulkan)`.
- The control protocol's existing `backend` token is unchanged.

**Non-Goals:**

- Full Chromium-style GPU process isolation for all rendering.
- macOS backend selection.
- Automatic driver updates.
- A System Info row or control-protocol field for the new label; only the
  status bar and About show it.
- Retrying a `crashed` backend automatically within the same app version.

## Decisions

### One backend per process, not a candidate set

`pictura-render` gains `GpuBackend { Dx12, Vulkan, Cpu }` and a
`OnceLock<GpuBackend>` pin. `request_device()` requests exactly
`backend.to_wgpu()`, one bit. `set_backend()` returns `Err(current)` once the
pin is resolved.

Alternative rejected: pass `DX12 | VULKAN` to one instance and let wgpu choose.
It loads both drivers, which is the crash. Alternative rejected: retry the next
backend in-process after a failure. `devices()` caches `None` on failure, and
retrying would initialize the next driver in the same process, so it is neither
possible nor safe. The chain is a launch-time selection, not a runtime fallback.

### Selection and probe orchestration in Rust; cache, marker, and UI in C++

`crates/pictura-render/src/gpu/select.rs` owns the pure parts: the platform
order, the compiled-backend intersection, `WGPU_BACKEND` parsing, the single
pin, `GpuBackendStatus`, and `backend_label()`. It is unit-testable without a
device. `resolve_candidates(platform, compiled, override)` is pure and returns
`(Vec<GpuBackend>, bool overridden)`; a thin wrapper reads the environment.

`crates/pictura-app/cpp/gpu_probe.{h,cpp}` owns the app concerns: the JSON
cache, the marker, and `prepareGpuBackend()`, which calls the Rust bridge for
candidate resolution, the child spawn, and the pin. It reuses `session.cpp`'s
path and `QSaveFile` conventions. `prepareGpuBackend` reads the candidates from
`gpu_candidates()`, calls `set_backend(kind)` before any `selected_backend()`
read, and sets safe mode only when a candidate is cached `crashed` (a driver
crash or a failed validation), not merely because the candidate list is
non-empty.

Alternative rejected: put candidate order in C++. The platform default and the
compiled-backend check are already Rust facts; duplicating them in C++ invites
drift and makes the pure tests harder.

### The probe is a child process; the Rust bridge spawns and classifies it

`pictura --gpu-probe <backend>` is handled in `main.cpp` before
`QApplication`, like `--mcp`. The child pins that backend and calls
`probe_backend_outcome()` (device creation plus the CPU-oracle validation
scene, reported as `Ok`/`NoAdapter`/`Invalid`). It is a fresh process, so the
`OnceLock`s are clean and no refactor of `Gpu::new` is needed.

The parent spawns the child with `std::process::Command`, polls `try_wait`
against a 10 s deadline, and kills on timeout. `std::process` uses
`CreateProcessW` and `TerminateProcess` on Windows, is portable, and lets the
timeout/kill logic run under the Linux test suite. `gpu_probe` takes the child
command or executable (a command factory) so tests spawn a controlled `sh -c` /
`cmd /C` child instead of the test binary. After a kill it reaps for up to
500 ms and then returns, so a Unix child is not left as a zombie and a driver
stuck in kernel mode cannot block the parent.

Alternative rejected: `QProcess` before `QApplication` (unsupported event-loop
use). Alternative rejected: Win32 `CreateProcessW` in C++ (Windows-only, not
testable on Linux, more code).

The child distinguishes three outcomes with `ProbeOutcome`: `Ok` (device created
and the CPU oracle matched) -> exit `0`, `NoAdapter` (uncompiled backend or no
device) -> exit `1`, and `Invalid` (a device exists but the CPU-oracle
validation failed) -> exit `2`. The Rust bridge returns `0` -> `ok`, `1` ->
`failed`, `2` -> `crashed` (validation failure, signal, access violation, or
timeout). C++ maps that outcome to the cache state and does not re-classify raw
exit codes. Windows may report an unfiltered access violation as a normal exit
with `0xC0000005`; the Rust bridge folds it into `2`.

### The child suppresses dialogs; the parent does not

The child sets `SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX |
SEM_NOOPENFILEERRORBOX)`, `SetThreadErrorMode`, and an unhandled-exception
filter that only calls `TerminateProcess` with exit code `2`, so the parent
records the crash as `crashed` rather than a clean failure. The filter must not
touch the CRT or the loader, because the crash can happen under the loader lock.
A stack-overflow variant cannot run the filter; the inherited error mode and the
parent's timeout are the backstop. The parent does not change its own error
mode.

### The cache is keyed by `PICTURA_VERSION`

`kooka-pictura/gpu-backend.json` sits beside `state.json`. Shape:

```json
{
  "schemaVersion": 1,
  "appVersion": "0.2.0",
  "backends": {
    "dx12": { "state": "ok" },
    "vulkan": { "state": "crashed" }
  }
}
```

Missing backend or unrecognised state reads as `unknown`. A different
`appVersion`, or a `schemaVersion` above 1, reads the whole file as `unknown`.
Writes are atomic via `QSaveFile`; the file is capped at 64 KiB and a corrupt
file reads as `unknown`. Unknown keys are not preserved across a
load-then-write.

The cache is hardware state, not UI state, so it does not go into `state.json`
and no chosen backend is persisted there. The `ok` entry is the persisted
choice.

### `crashed` is sticky for the app version; `failed` retries

`ok` is selected without a probe. `failed` is eligible again on the next launch.
`crashed` is skipped until the app version changes or the user re-enables "Use
GPU Compute" (which deletes the cache). The `WGPU_BACKEND` override replaces the
platform candidate order and still runs the probe; it does not bypass a
`crashed` entry, so a forced backend cannot produce a silent crash loop. This is
the smallest rule that cannot produce a silent crash loop.

### The marker is a single write/delete

`gpu-init.marker` contains `{backend, appVersion, pid}`. It is written before
the parent's `gpu_available()` and deleted after it returns. A marker present at
startup for the same version marks that backend `crashed` and is removed, unless
its recorded PID is a live process: a live marker belongs to a concurrent
instance and is ignored without a cache change. `beginDeviceInit` refuses to
overwrite a marker whose PID is a live, different process, so a concurrent
instance's crash record is not clobbered. A malformed marker, or one with a
future `schemaVersion`, is removed without a cache change. `tst_gpu_backend`
uses `ScopedStateHome` so the cache and marker tests never touch the developer's
state directory or race another suite.

`endDeviceInit(false)` records `failed` and `endDeviceInit(true)` records `ok`;
either clears the marker, so the next launch does not trust a stale `ok`.

The false-positive window is a crash after `gpu_available()` returns but before
the delete. It would mark a good backend `crashed` for the version, recoverable
through Preferences. Closing it needs a two-phase `ok` marker or a lock, which
is more state than this change needs.

`ponytail: single marker, no lock; add a QLockFile or a two-phase marker if a
concurrent-instance false positive is observed.`

### Five labels; the control protocol keeps its legacy token

`active_backend()` keeps returning `GPU`, `CPU`, `CPU (no GPU)` so
`status.backend` and `set_gpu_compute.backend` do not change. A new
`active_backend_label()` returns `GPU (DX12)`, `GPU (Vulkan)`, `CPU`,
`CPU (no GPU)`, or `CPU (safe mode)` for the status bar and About.

Precedence: no adapter -> `CPU (no GPU)`; user disabled -> `CPU`; blocked ->
`CPU (safe mode)`; otherwise the active backend's API name. The API name comes
from the selected backend, not `gpu_adapter_info()`, which returns `None` after
a device loss.

Alternative rejected: change `active_backend()` and accept the control-protocol
break. `selftest.cpp:1340-1350` enforces the legacy token, so keeping it avoids
a needless self-test change; there is no need to break it.

### The Preferences toggle is enabled by candidate availability

`gpuAvailable_` gates actual compositing and is false in safe mode. The
checkbox is enabled from a new `gpuCandidatesAvailable_`, which is true when an
`ok`/`unknown` candidate exists or the chain ended with a cached `crashed`
entry (a driver crash or a failed validation), and false when every candidate
cleanly `failed` or no backend is compiled. A clean no-adapter machine therefore
reports `CPU (no GPU)` with the toggle disabled; safe mode keeps the reset path.
Turning the preference off then on deletes the cache.

### DX12 is compiled on Windows only

Add the feature through a target-scoped dependency in both manifests:

```toml
[target.'cfg(windows)'.dependencies]
wgpu = { version = "30.0.1", default-features = false,
         features = ["std", "dx12", "vulkan", "wgsl"] }
```

Linux's dependency graph is unchanged. `compiled_backends()` uses
`enabled_backend_features()`, which follows wgpu's own `cfg(dx12)` and is
therefore false on Linux even if the feature is on.

### Interop stays Vulkan-only

`create_interop_state()` calls `create_device(GpuBackend::Vulkan)` explicitly
and guards `is_compiled()`. `--interop-probe` keeps its early path and does not
call `prepareGpuBackend()`, so the chain cannot select it.

### Windows CI is a dedicated job

`ci.yml` gains a `render` path filter and a `windows-render` job on
`windows-2022` running `cargo clippy -p pictura-render --all-targets -- -D
warnings` and `cargo test -p pictura-render --lib --test backend_pin --test
backend_cpu`. It reuses the release
workflow's action pins and sccache v0.18.0. The gate is the precise filter, not
`cpp_only`, so unrelated changes do not spend Windows minutes. The job is scoped
to `pictura-render` per issue #1086; the app and C++ shell are covered by the
release `windows.yml` job, which runs on release PRs and tags only.

## Risks / Trade-offs

- **One-time parent crash after a successful child probe** -> The marker records
  the attempt; the next launch skips that backend. The first launch is lost, as
  the proposal states.
- **Parent clean failure after a child `ok`** -> `endDeviceInit(false)` records
  `failed`, so the next launch does not trust the stale `ok` and tries the next
  candidate.
- **Concurrent instances and a stale-looking marker** -> A live PID is ignored,
  so a concurrent instance is not mistaken for a crash. A crash between the
  probe returning and the delete can still mark a good backend `crashed` for the
  version, cleared from Preferences.
- **The child probe may not reproduce the Mozilla loader stack overflow** ->
  The probe is a crash-isolation harness, not a deterministic reproducer. The
  error mode and timeout are the backstop.
- **WARP is rejected** -> `accelerates()` treats `DeviceType::Cpu` as no GPU, so
  a GPU-less Windows runner reports a clean `failed`, not a crash. Do not weaken
  the production gate for the probe.
- **A driver stuck in kernel mode may ignore `TerminateProcess`** -> The parent
  reaps for up to 500 ms after `kill()` and then returns, so it still exits; the
  child may linger until the driver returns.
- **`dx12` pulls Windows-only transitive crates into the lockfile** -> Run
  `cargo deny check` and regenerate the Flatpak source list if the gate reports
  a diff. Linux compile graph is unchanged.
- **Exact-string self-test** -> `selftest.cpp:1340-1350` keeps the legacy
  `active_backend()` token, so it needs no edit; new GUI checks go to
  `tst_gpu_backend`.
- **`gpu_adapter_info()` returns `None` after device loss** -> The status uses
  the selected backend and `device_lost()`/the safe-mode flag, not the adapter
  info.
- **The Windows `render` job does not compile `pictura-app` or the C++ shell**
  -> The app/C++ changes are compiled by the release `windows.yml` job, which
  runs on release PRs and tags only. This is an accepted trade-off, recorded
  here so the gap is not silent.
- **Windows CI cold cache** -> ~15-25 min cold, ~5-10 min warm; `timeout-minutes:
  45`.

## Migration Plan

No data migration: a cache from another version reads as `unknown`, and the
marker is ignored across versions. Reverting touches the same files: it removes
the selection module, the `dx12` feature, and the probe files, and restores the
Vulkan-only device paths; the CPU oracle and compositor signatures are
unchanged, and Linux never used the chain.

## Open Questions

None that change the specs, approach, or task breakdown. Two follow-ups are
deferred and recorded here: a lock or two-phase marker if concurrent-instance
false positives are observed, and a System Info row if users need the label
outside the status bar and About.
