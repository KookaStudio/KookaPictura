# Proposal: System Info panel

## Why

`Help > System Info` exists as an unwired leaf, so the application cannot tell a
user (or a bug report) what it is running on. Issue #815 asks for a diagnostics
surface that reports the OS, CPU, GPU, memory, process usage, displays, and the
Pictura build, plus a `Submit Issue` entry that carries that report into a new
GitHub issue.

## What changes

- `Help > System Info` opens a modeless dialog with a plain-text environment
  report and a `Copy` button. The report is a snapshot taken when the dialog
  opens; reopening refreshes it.
- `Help > Submit Issue`, directly below `System Info`, opens the repository's
  new-issue page in the default browser with the report in the issue body. The
  report is also placed on the clipboard, and a failed browser launch falls
  back to a message that says so.
- Linux collects CPU model, physical/logical cores, clock, RAM/swap, system CPU
  usage, and the process's own RAM/CPU from `/proc`. Qt supplies the OS, kernel,
  architecture, desktop/session hints, and display geometry. The renderer's
  wgpu adapter supplies the GPU name, device type, backend, and driver.
  Unsupported metrics render `Unavailable` instead of failing.
- The report never includes the hostname, user name, monitor serial numbers,
  network addresses, environment variables, or document paths.
- The application version and build type are exposed to C++ from the existing
  CMake project version, so the report and future About work share one source.

## Capabilities

- Added: `ui/system-info`.

## Impact

- `pictura-render`: expose the cached adapter's `wgpu::AdapterInfo` as a small
  read-only summary.
- `pictura-app` Rust bridge: one `gpuAdapterInfo()` invokable returning the
  adapter fields.
- `pictura-app` C++: `system_info.{h,cpp}` (collector, report formatter, issue
  URL), `system_info_dialog.{h,cpp}`, `frame_system_info.cpp`, Help-menu wiring,
  and CMake build/version definitions.
- Tests: new `tst_system_info` Qt Test suite plus a Rust adapter-summary unit.

## Out of scope

- The Preferences Performance pane, Efficiency/Scratch status readouts, and
  scratch-disk configuration tracked by #636. This change does not re-scope
  that issue.
- The About dialog's version/build/link layout tracked by #367. This change
  only shares the version source.
- VRAM size and GPU clock: wgpu does not expose them; they render
  `Unavailable`.
- Exact desktop build versions and built-in/external monitor types: no
  portable API; the report gives the desktop/session name and the monitor's
  reported model, resolution, size, DPI, and refresh rate.
- Windows/macOS-only CPU and process metrics beyond Qt's OS/architecture/
  display data; those rows render `Unavailable`.
