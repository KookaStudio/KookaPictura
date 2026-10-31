# agent-permissions Specification

## Purpose
What an agent attached to a control session may do, and how a person in a live window stays in charge of their work and files.

## Requirements

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

### Requirement: Agent Control preferences

Edit > Preferences > Agent Control SHALL offer the enable toggle, the status and
connected client count, the Edit and Save permissions, the approval mode, allowed
folders, and a copyable MCP client configuration. A change SHALL apply at once and
persist. Save SHALL NOT be selectable without Edit. Allow all SHALL show a warning.
The page SHALL state that permissions constrain agents using this interface and not
one that also has a shell. Under Flatpak or Snap the page SHALL work, SHALL NOT offer
one-click client registration, and SHALL say the configuration is added by hand;
under Flatpak it SHALL also say which files the app can see. Stopping the server
SHALL withdraw a pending approval.

#### Scenario: Enabling starts the server [apf_enable]

- **WHEN** the person ticks Enable, then unticks it
- **THEN** the server runs, then stops, and the status says so

#### Scenario: Permissions apply at once [apf_permissions]

- **WHEN** the person ticks Edit, Save, and picks Manual on a running server
- **THEN** the running policy has them and a new page shows them

#### Scenario: Save needs Edit [apf_save_needs_edit]

- **WHEN** Edit is unticked
- **THEN** Save is unticked and disabled

#### Scenario: Allow all warns [apf_allow_all]

- **WHEN** Allow all is chosen
- **THEN** the warning shows, and hides for any other mode

#### Scenario: Folders persist [apf_folders]

- **WHEN** a folder is added and another removed
- **THEN** the settings hold the remaining one

#### Scenario: The client snippet is valid [apf_snippet]

- **WHEN** the snippet is read as JSON
- **THEN** it names the command a client outside any sandbox runs: this
  executable or AppImage with `--mcp`, or `flatpak run` or `snap run` with the
  app's name and `--mcp`

#### Scenario: The limits are stated [apf_limits]

- **WHEN** the page is shown
- **THEN** it says a shell is outside the permissions

#### Scenario: Sandboxed builds explain themselves [apf_sandbox]

- **WHEN** the app runs under Flatpak or Snap
- **THEN** the controls are enabled, no registration box is offered, and a note
  says to add the configuration by hand, and under Flatpak names `flatpak override`

#### Scenario: Stopping withdraws an approval [apf_stop_pending]

- **WHEN** a request waits for approval and the server is stopped
- **THEN** the bar hides

#### Scenario: The menu opens the page [apf_menu]

- **WHEN** Edit > Preferences > Agent Control is invoked
- **THEN** the dialog opens on the page

### Requirement: Agent control is off until enabled in every build

The running application SHALL NOT serve a live control endpoint unless agent
control is enabled in the session store; it SHALL default to off with Read access
only. It SHALL be able to start and stop while running, and SHALL persist its
settings. A second window SHALL report that another window is serving and SHALL
leave that endpoint untouched. Flatpak and Snap builds SHALL serve it from a
directory shared by the sandbox's instances, and SHALL NOT need a sandbox
permission for it. Crash recovery SHALL stay on during a live session.

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
