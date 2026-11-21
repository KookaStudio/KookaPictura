# Spec Delta

## MODIFIED Requirements

### Requirement: Agent control is off until enabled in every build

The running application SHALL NOT serve a live control endpoint unless agent
control is enabled in the preferences store; it SHALL default to off with Read access
only. It SHALL be able to start and stop while running, and SHALL persist its
settings. A second window SHALL report that another window is serving and SHALL
leave that endpoint untouched. Flatpak and Snap builds SHALL serve it from a
directory shared by the sandbox's instances, and SHALL NOT need a sandbox
permission for it. Crash recovery SHALL stay on during a live session.

#### Scenario: Off by default [ap_off_by_default]

- **WHEN** a preferences store has no agent control keys
- **THEN** no endpoint exists

#### Scenario: Started and stopped at runtime [ap_live_start_stop]

- **WHEN** agent control is enabled, then disabled
- **THEN** a client can connect after the first, and the endpoint and token are
  gone after the second

#### Scenario: Settings persist [ap_live_settings_persist]

- **WHEN** settings are changed and a new `AgentControl` loads the store
- **THEN** it has the same settings

#### Scenario: A second window does not take over [ap_second_window]

- **WHEN** a second `AgentControl` starts while one is serving
- **THEN** it reports the conflict and the first keeps serving

#### Scenario: A dead holder does not block [ap_dead_holder]

- **WHEN** a lock file is left behind by a process that is gone
- **THEN** agent control starts

#### Scenario: Sandboxed builds start [ap_sandbox]

- **WHEN** the app runs under Flatpak or Snap and agent control is enabled
- **THEN** it serves

#### Scenario: Flatpak instances share the endpoint [ap_sandbox_endpoint]

- **WHEN** the app runs under Flatpak
- **THEN** the endpoint is under `$XDG_RUNTIME_DIR/app/$FLATPAK_ID`

#### Scenario: Other builds keep their endpoint [ap_endpoint_outside_flatpak]

- **WHEN** the app runs under Snap or outside a sandbox
- **THEN** the endpoint is under `$XDG_RUNTIME_DIR/pictura`

#### Scenario: A packaged build answers a client [ap_sandbox_round_trip]

- **WHEN** a window serves in an installed Flatpak or strictly confined Snap, and
  the advertised command runs outside the sandbox
- **THEN** it completes an MCP exchange with that window, with only protocol on
  stdout

### Requirement: File access is confined and approved

A restricted session SHALL judge the canonical target of `open`, `save`, and
`save_as`. The state directory, configuration directory, and endpoint directory SHALL be refused in
every mode. Manual SHALL ask for a target outside the allowed folders and for any
overwrite. Auto-approve SHALL ask the same, except for a file the agent made or
the person approved this session. Allow all SHALL never ask. A headless session
SHALL refuse wherever a live one would ask.

#### Scenario: Inside an allowed folder [ap_path_roots]

- **WHEN** a live session opens a file inside an allowed folder
- **THEN** it opens without asking

#### Scenario: Headless refuses outside [ap_path_outside_headless]

- **WHEN** a headless session opens a file outside its roots
- **THEN** the response is `refused`

#### Scenario: A symlink does not escape [ap_path_symlink_escape]

- **WHEN** a directory inside a root links outside it, and a path goes through it
- **THEN** the path is judged as outside

#### Scenario: A sibling prefix is not inside [ap_path_sibling_prefix]

- **WHEN** the root is `/a/b` and the target is `/a/b2/x.psd`
- **THEN** the target is judged as outside

#### Scenario: Protected directories are refused in every mode [ap_hard_deny]

- **WHEN** Allow all targets the state directory
- **THEN** the verdict is Refuse

#### Scenario: Approval modes decide as tabled [ap_approval_matrix]

- **WHEN** each mode judges inside, outside, overwrite, and known targets
- **THEN** each verdict matches the mode's rule

#### Scenario: Separate configuration root is protected [ap_config_hard_deny]

- **WHEN** restricted live and CLI control attempt open, save, or save_as under a configuration root separate from the state and endpoint roots, including in Allow all mode
- **THEN** each request is refused, including canonical paths reached through a symlink

