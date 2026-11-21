# Proposal

## Why

Issue #649 tracks two remaining gaps in identifying a running build. Scripts
have no `--version` command, and the About dialog shows a renderer label and
generic Qt branding without the exact Qt version or GPU adapter details.

## What Changes

- Add `pictura --version` with one line on stdout sourced from the same
  application version as the desktop UI. The command works without a display.
  Windows scripts can pipe or redirect stdout from the packaged executable.
- Show the build type, Qt runtime version, GPU adapter name, and GPU driver in
  About alongside the app version, package method, and active renderer. Show
  `Unavailable` where adapter information is missing.
- Record that the stable update checker shipped with #1146 and update the
  roadmap entry for #649 when the CLI and About changes ship. The draft's beta
  and nightly feeds require their own release and packaging decision.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/application-shell` gains the display-free application version command.
- `ui/update-preferences` extends the existing About version and package
  readout requirement with build and runtime diagnostics.

## Impact

- C++ application startup, the existing About command, and the text it
  displays. System Info already has the relevant Qt and GPU data sources.
- Qt Test coverage for the executable's CLI output and for the About dialog
  opened from a window without a document, plus a PowerShell smoke check for
  the staged Windows executable.
