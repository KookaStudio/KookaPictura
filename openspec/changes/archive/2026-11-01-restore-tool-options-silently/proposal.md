# Proposal

## Why

`tool_options_json.h` documents `applyToolOptionsJson` as restoring the session
store "without notifying", but the brush and selection groups restore through
their normal setters, so `sizeChanged`, `tipChanged`, `paintSettingsChanged`,
`combineModeChanged`, and `magneticWidthChanged` fire during startup. That is
harmless only because the options bars are built after `buildTools` restores the
state; any listener attached earlier would repaint or overwrite the restored
value against a document that is not ready (#713).

The same function also merges a missing group or field into the value the group
already holds, while the header and the persistence spec promise that a missing
entry keeps the default. The control server reuses the function as a runtime
patch, where merging into the current value is the documented behaviour.

## What Changes

- **Silent restore.** `PaintOptions`, `SelectionOptions`, and the Crop settings
  gain a restore path that assigns every persisted field with the setter's
  clamping and without a change callback. `applyToolOptionsJson` uses it, and
  reads each present group from a default-constructed value, so a missing group
  or field restores its default.
- **Separate patch.** `patchToolOptionsJson` keeps the control server's
  merge-into-current behaviour and still notifies for the groups whose setters
  notify. `ControlServer::methodToolOptions` calls it.
- **Contract.** The header states the two behaviours separately, and the
  persistence and agentic-control specs record them.

## Capabilities

### Modified Capabilities

- `ui/tool-options-persistence`: restore is silent and a missing group or field
  restores the default.
- `interop/agentic-control`: `tool_options` merges into the current values.

## Impact

- `crates/pictura-app/cpp/tool_options.h`,
  `cpp/tool_options_json.{h,cpp}`, `cpp/control_server_actions.cpp`.
- `crates/pictura-app/cpp/tests/tst_tool_options_persistence.cpp` and
  `cpp/tests/tst_crop_settings.cpp`.
- No engine, codec, or dependency change.
