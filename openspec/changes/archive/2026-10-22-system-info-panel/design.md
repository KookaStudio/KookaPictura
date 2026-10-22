# Design: System Info panel

## Shape

Three small pieces keep the platform reads, the presentation, and the frame
wiring separable:

- `system_info.{h,cpp}` — a `SystemInfo` value, the collector, pure parse and
  format helpers, and `submitIssueUrl`.
- `system_info_dialog.{h,cpp}` — a read-only `QPlainTextEdit` over the formatted
  report, `Copy` and `Close`.
- `frame_system_info.cpp` — the two Help handlers, the modeless dialog cache,
  and the clipboard/browser fallback.

The dialog takes a `SystemInfo` value, not a live probe, so tests can render a
fixture without touching `/proc`, the GPU, or the display server.

## Menu and commands

`Help > System Info` already exists as a path-derived leaf
(`help.system.info`). It becomes a frozen `command_ids::HelpSystemInfo` with a
handler, so its id and position survive. `Help > Submit Issue` is added
directly below it as `command_ids::HelpSubmitIssue` (`help.submit.issue`). Both
are `implemented=true`; neither requires a document.

`Help > System Info` creates a `PictureView` probe (the same pattern the frame
uses at startup for the GPU preference), asks it for the adapter fields, and
opens the dialog. A second invocation refreshes the existing dialog and raises
it rather than stacking dialogs.

`Help > Submit Issue` collects a fresh report, copies it to the clipboard,
builds the GitHub URL, and calls `QDesktopServices::openUrl`. When that returns
false it shows one warning naming the clipboard fallback. There is no network
code in the application.

## GPU information

`pictura-render::gpu` keeps the cached `wgpu::Adapter` alive in `Devices`. Add
`Devices::adapter_info()` and a public `gpu_adapter_info()` returning:

```rust
pub struct GpuAdapterInfo {
    pub name: String,
    pub device_type: String,
    pub backend: String,
    pub driver: String,
    pub driver_info: String,
}
```

The strings are the `Debug` names of `wgpu::DeviceType` / `wgpu::Backend` so the
bridge does not depend on wgpu types. The function returns `None` when no
usable adapter exists. The Rust bridge exposes the five fields as a `QStringList`
in that order; an empty list means no GPU. VRAM is not available from wgpu and
renders `Unavailable`.

## Linux metrics

All `/proc` reads are best-effort and return `Unavailable` on failure:

- `/proc/cpuinfo`: `model name` (or `Hardware`/`Processor` on ARM), unique
  `(physical id, core id)` pairs for physical cores, `cpu MHz` values for the
  current clock, and `cpuinfo_max_freq` from `/sys/devices/system/cpu/cpu0/
  cpufreq/` for the ceiling.
- `/proc/meminfo`: `MemTotal`, `MemAvailable` (falling back to `MemFree` plus
  `Buffers` plus `Cached`), `SwapTotal`, `SwapFree`.
- `/proc/stat`: the first `cpu` line sampled twice 200 ms apart for current
  system CPU usage.
- `/proc/self/status`: `VmRSS` for Pictura's RAM.
- `/proc/self/stat` plus `/proc/uptime`: process CPU ticks sampled twice for
  the current percentage, and total ticks since `starttime` for the average
  since launch.

The 200 ms sample is the only deliberate wait; it runs when the dialog opens or
Submit Issue is chosen, never during normal editing. Parsers are free functions
so tests can feed fixture text.

## Report format

Plain text with fixed section headings (`Application`, `System`, `CPU`,
`Memory`, `GPU`, `Displays`) and `Label: value` rows. Every absent value is
`Unavailable`; the report never prints a blank row. The same string feeds the
dialog, the clipboard, and the issue body, so what the user sees is what is
submitted.

`submitIssueUrl` uses:

```
https://github.com/KookaStudio/KookaPictura/issues/new?body=<percent-encoded>
```

The body is a short `### System Info` heading plus a fenced block. The report
is capped at 6000 characters; a longer report is truncated with an explicit
marker, and the full report remains on the clipboard. The URL is built with
`QUrl`/`QUrlQuery`, not string concatenation.

## Privacy

The collector reads only the fields listed above. It does not read the hostname,
user name, serial numbers, network interfaces, environment variables beyond the
desktop/session hints, or document paths. The desktop/session hints are names
such as `GNOME` or `wayland`, not identifiers.

## Version and build

CMake adds `PICTURA_VERSION="${PROJECT_VERSION}"` and
`PICTURA_BUILD_TYPE="$<CONFIG>"` to the shell's compile definitions. The report
uses `PICTURA_VERSION` (falling back to `unknown`), the build type (falling
back to `unknown`), and `qVersion()`. No git hash is read at configure time,
because release tarballs may not carry `.git`; the version remains the single
source release-please already bumps.

## Testing

- C++ Qt Test `tst_system_info` covers the report sections and fallbacks, the
  dialog text and clipboard, refresh, the issue URL and its cap, the `/proc`
  parsers, byte formatting, privacy, and the Help wiring with no document.
- A Rust unit checks that `gpu_adapter_info()` either returns a non-empty name
  or `None` and never panics.
- Opening the default browser is a manual row: offscreen Qt Test cannot observe
  the desktop's URL handler.

## Ceilings

- `ponytail:` Windows and macOS report Qt's OS/architecture/display data and
  `Unavailable` for `/proc`-derived CPU, memory, and process metrics.
- `ponytail:` CPU usage is a 200 ms sample, not a live meter; the dialog is a
  snapshot, not a monitoring panel.
- `ponytail:` VRAM, GPU clock, desktop build version, and built-in/external
  monitor type are not exposed by the portable APIs in use.
