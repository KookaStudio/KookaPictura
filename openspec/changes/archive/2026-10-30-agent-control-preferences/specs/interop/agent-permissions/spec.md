## ADDED Requirements

### Requirement: Agent Control preferences

Edit > Preferences > Agent Control SHALL offer the enable toggle, the status and
connected client count, the Edit and Save permissions, the approval mode, allowed
folders, and a copyable MCP client configuration. A change SHALL apply at once and
persist. Save SHALL NOT be selectable without Edit. Allow all SHALL show a warning.
The page SHALL state that permissions constrain agents using this interface and not
one that also has a shell. Under Flatpak or Snap the page SHALL be disabled and SHALL
say why. Stopping the server SHALL withdraw a pending approval.

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
- **THEN** it names this executable and `--mcp`

#### Scenario: The limits are stated [apf_limits]

- **WHEN** the page is shown
- **THEN** it says a shell is outside the permissions

#### Scenario: Sandboxed builds explain themselves [apf_sandbox]

- **WHEN** the app runs under Flatpak
- **THEN** the controls are disabled and the reason is shown

#### Scenario: Stopping withdraws an approval [apf_stop_pending]

- **WHEN** a request waits for approval and the server is stopped
- **THEN** the bar hides

#### Scenario: The menu opens the page [apf_menu]

- **WHEN** Edit > Preferences > Agent Control is invoked
- **THEN** the dialog opens on the page
