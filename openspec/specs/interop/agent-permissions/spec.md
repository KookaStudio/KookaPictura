# agent-permissions Specification

## Purpose
What an agent attached to a control session may do, and how a person in a live window stays in charge of their work and files.

## Requirements

### Requirement: Agent control is off until enabled

The running application SHALL NOT serve a live control endpoint unless agent
control is enabled in the session store; it SHALL default to off with Read access
only. It SHALL be able to start and stop while running, and SHALL persist its
settings. A second window SHALL report that another window is serving and SHALL
leave that endpoint untouched. Flatpak and Snap builds SHALL NOT start it. Crash
recovery SHALL stay on during a live session.

#### Scenario: Off by default [ap_off_by_default]

- **WHEN** a session store has no agent control keys
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

#### Scenario: Sandboxed builds do not start [ap_sandbox]

- **WHEN** the app runs under Flatpak or Snap
- **THEN** starting agent control fails with a reason

### Requirement: Every method has a permission tier

Each control method SHALL have one of the tiers Read, Edit, Save, or Admin,
enforced by the control server. A restricted session SHALL answer `refused` for
a tier it does not hold. Edit SHALL require the Edit grant. Save SHALL require
both the Save and Edit grants. Admin methods, which include `key` and `pointer` in
window space, SHALL be refused in every restricted session. Outside bare
`--control`, `dispatch_command` SHALL run only a command on a fixed list.

#### Scenario: Read-only refuses edits [ap_tier_read_only]

- **WHEN** a session without the Edit grant calls `filter`
- **THEN** the response is `refused`, and `status` still works

#### Scenario: Edit without Save refuses saving [ap_tier_edit_no_save]

- **WHEN** a session with Edit but not Save calls `save_as`
- **THEN** the response is `refused` and no file is written

#### Scenario: Every method has a tier [ap_every_method_has_tier]

- **WHEN** the method table is enumerated
- **THEN** every name has a tier, and `key` and `set_unsaved_policy` are Admin

#### Scenario: Admin methods are refused [ap_admin_refused]

- **WHEN** a restricted session with every grant calls `key`, or `pointer` with
  `space` `window`
- **THEN** the response is `refused`

#### Scenario: Commands are default-deny [ap_command_allowlist]

- **WHEN** a restricted session dispatches `file.exit`, and then `select.all`
- **THEN** the first is `refused` and the second runs

#### Scenario: Bare control is unchanged [ap_bare_control]

- **WHEN** a client of an unrestricted server calls `key`
- **THEN** the tier check does not refuse it

### Requirement: File access is confined and approved

A restricted session SHALL judge the canonical target of `open`, `save`, and
`save_as`. The state directory and the endpoint directory SHALL be refused in
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

### Requirement: Approval never blocks the application

A request needing approval SHALL wait without a modal dialog and without blocking
the GUI thread; later requests SHALL wait behind it. The bar SHALL show the
canonical path as plain text and SHALL NOT take keyboard focus. Allow SHALL run
the request after repeating every check. Deny SHALL answer `refused`. After 60
seconds it SHALL answer `approval_timeout`. A client disconnect SHALL withdraw it.

#### Scenario: A request waits for approval [ap_prompt_parks]

- **WHEN** a live session requests a target that asks, then sends `status`
- **THEN** the bar shows, neither gets a reply, and the window stays responsive

#### Scenario: Allow runs the request [ap_prompt_allow]

- **WHEN** the person allows
- **THEN** the request runs and both replies arrive in order

#### Scenario: Deny refuses the request [ap_prompt_deny]

- **WHEN** the person denies
- **THEN** the response is `refused` and nothing is written

#### Scenario: An unanswered prompt times out [ap_prompt_timeout]

- **WHEN** nobody answers before the timeout
- **THEN** the response is `approval_timeout` and the bar hides

#### Scenario: A disconnect withdraws the prompt [ap_prompt_disconnect]

- **WHEN** the client disconnects while waiting
- **THEN** the bar hides and later clients are served

### Requirement: A live session protects the person's work

In a live session a `close` or `revert` of a dirty document SHALL be answered
`unsaved` and SHALL NOT discard anything. An unsaved prompt reached by a request
SHALL resolve to Cancel for that request only, so a person's own Close still
prompts. Edit and Save requests SHALL be answered `human_busy` while a dialog,
popup, held mouse button, pending transform or crop, or loading document is
active. Over a socket, `open` SHALL NOT cancel a dialog the person opened.

#### Scenario: A dirty close is refused [ap_live_dirty_close]

- **WHEN** a live session closes a dirty document
- **THEN** the response is `unsaved` and the document stays open

#### Scenario: A person's prompt is unaffected [ap_human_close_prompts]

- **WHEN** a request resolved an unsaved prompt
- **THEN** the next prompt outside a request uses the normal policy

#### Scenario: Edits wait for a person [ap_human_busy]

- **WHEN** a dialog is open and a live session calls `filter`
- **THEN** the response is `human_busy`, and `status` still works

#### Scenario: Open leaves a person's dialog alone [ap_open_deferred]

- **WHEN** a person has a dialog open during a socket `open`
- **THEN** the open succeeds and the dialog stays open
