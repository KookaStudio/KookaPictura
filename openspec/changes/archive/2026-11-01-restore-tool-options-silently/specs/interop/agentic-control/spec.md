## MODIFIED Requirements

### Requirement: Tool options readback and patch

The server SHALL provide a `tool_options` method. Given an `options` object it
SHALL merge it into the current tool options (a group or field it leaves out
keeps its current value, each value is clamped as its setter would), and with or
without one it SHALL return `{options}`, every tool's options as the session
store writes them. An `options` that is not an object SHALL be `invalid_param`.

#### Scenario: A brush size is patched and read back [ctl_tool_options]

- **WHEN** a client sends `tool_options` with `{"brush": {"size": 321}}`, then
  without options, then with `options` set to a number
- **THEN** the first response reports brush size 321 and the Brush uses it, the
  second reports the options, and the third is `invalid_param`

#### Scenario: A patch keeps the fields it omits [ctl_tool_options_merge]

- **WHEN** `tst_tool_options_persistence::patchMergesAndNotifies` patches one
  brush field on a controller whose other brush fields are not defaults
- **THEN** the patched field changes, the omitted fields keep their values, and
  the change listener fires
