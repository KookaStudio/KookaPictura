## REMOVED Requirements

### Requirement: Agent control is off until enabled

**Reason**: It refused Flatpak and Snap builds, which now serve.

**Migration**: Replaced by "Agent control is off until enabled in every build".

## ADDED Requirements

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

## MODIFIED Requirements

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
