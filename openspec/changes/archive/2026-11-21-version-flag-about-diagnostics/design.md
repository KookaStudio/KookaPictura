# Design

## Context

See the proposal for the scope of #649. The desktop shell already gets
`PICTURA_VERSION` and `PICTURA_BUILD_TYPE` from CMake. The Help handler composes
About from `aboutText(activeBackendLabel(gpuCompute_))` and `aboutVersionText()`.
System Info already reads `qVersion()` and the selected adapter through
`PictureView::gpu_adapter_info()`.

## Goals / Non-Goals

- Give scripts a stable version line and give support enough information to
  identify the running build and graphics environment.
- Treat codec and ABI inventories, Git build stamps, and prerelease feeds as
  separate work. The release pipeline currently publishes the version already
  used by the stable updater.

## Decisions

1. **Handle the standalone flag in the pre-application argument path.** Check
   the raw arguments at the start of `main()`, ahead of the `--gpu-probe`
   scan, the MCP bridge, and `QApplication`. Print `PICTURA_VERSION` to stdout.
   The parser that uses `QApplication::arguments()` runs too late to serve a
   script without a display or a usable Qt platform plugin.
   Keep the output to the product name and version so callers can parse it
   consistently across locales and builds.
2. **Extend the existing About message.** Keep the text supplied by
   `aboutVersionText()` for the version and package. Use `qVersion()` for the
   Qt runtime and `PICTURA_BUILD_TYPE` for the build configuration. Compose
   those details alongside the current renderer label and preserve the
   About dialog's licensing and trademark text. Keep the existing action and
   menu-role behavior, including macOS Application Menu placement.
3. **Read the adapter summary from the existing renderer bridge.** The frame
   has already performed its startup GPU probe, and the adapter summary reads
   the cached device result. Use the adapter name and the available driver
   name and details. Read these fields even when the user disables GPU compute
   so support can see the hardware alongside the active CPU renderer. The full
   System Info collector includes OS sampling and display enumeration, which
   About does not need. Keep the About formatting testable with fixture values.

## Risks / Trade-offs

- A compile-time Qt version could disagree with the loaded library. Report the
  runtime version even when the packaging metadata names a different build.
- A device can be present when rendering is on the CPU. Use separate fields and
  fill missing values individually with `Unavailable` to preserve partial data.
- Use the built `pictura` target for the `QProcess` test. Keep the Windows GUI
  subsystem and use inherited stdout for piped or redirected invocation.
  Verify the supported invocation with a PowerShell pipe from the staged
  Windows binary. Compare the output with the checkout's `version.txt`, which
  tracks the CMake project version. Windows PR packaging can use a synthetic
  version in artifact names.
