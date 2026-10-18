## ADDED Requirements

### Requirement: Tool options readback and patch

The server SHALL provide a `tool_options` method. Given an `options` object it
SHALL merge it into the tool options exactly as the session store restores
them (a group or field it leaves out keeps its value, each value is clamped as
its setter would), and with or without one it SHALL return `{options}`, every
tool's options as the session store writes them. An `options` that is not an
object SHALL be `invalid_param`.

#### Scenario: A brush size is patched and read back [ctl_tool_options]

- **WHEN** a client sends `tool_options` with `{"brush": {"size": 321}}`, then
  without options, then with `options` set to a number
- **THEN** the first response reports brush size 321 and the Brush uses it, the
  second reports the options, and the third is `invalid_param`

### Requirement: Held-button pointer steps

The `pointer` method SHALL also accept `op` `down` and `up`, a button press and
release at `x`/`y`. Between a `down` and its `up`, a `move` SHALL carry the
pressed button, so a client can send each motion of a drag as its own request
and time it; any other `move` carries no button.

#### Scenario: A drag sent a step at a time paints [ctl_pointer_steps]

- **WHEN** a client selects the Brush, sends `pointer` `down`, several `move`s,
  and `up` in image space, then one more `move`
- **THEN** one stroke is recorded along the moves, and the last `move` paints
  nothing
