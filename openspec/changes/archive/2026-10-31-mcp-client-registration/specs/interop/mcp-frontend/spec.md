## ADDED Requirements

### Requirement: Registering with MCP clients

The Agent Control page SHALL offer Register and Unregister for each supported
client. Nothing SHALL be written until the person accepts a change that shows the
result. A JSON configuration SHALL change only under `mcpServers`, SHALL be backed up
first, and SHALL be written atomically. A file that cannot be parsed SHALL be refused
and left untouched. Replacing a different `pictura` entry SHALL need its own
acceptance. A symbolic link SHALL be written through and kept. A client with a
command-line tool SHALL be changed through that tool, and SHALL be reported as
unavailable when it is not found.

#### Scenario: A new file is created [reg_new_file]

- **WHEN** a client has no configuration and the person registers
- **THEN** the file holds the `pictura` entry

#### Scenario: Other servers survive [reg_merge]

- **WHEN** a configuration has other servers and keys
- **THEN** after registering they are unchanged and a backup holds the original

#### Scenario: Invalid files are refused [reg_invalid]

- **WHEN** a configuration is invalid JSON, or `mcpServers` is not an object
- **THEN** registration reports a problem and the file is unchanged

#### Scenario: Declining changes nothing [reg_decline]

- **WHEN** the person declines
- **THEN** no file is written

#### Scenario: A different entry needs its own acceptance [reg_replace]

- **WHEN** a `pictura` entry names another command
- **THEN** the question names the replacement

#### Scenario: Unregistering removes only the entry [reg_unregister]

- **WHEN** the person unregisters
- **THEN** only the `pictura` entry is gone

#### Scenario: A symbolic link is kept [reg_symlink]

- **WHEN** the configuration is a link to another file
- **THEN** the target changes and the link remains

#### Scenario: The CLI is used where there is one [reg_cli]

- **WHEN** the person registers Claude Code
- **THEN** the tool runs with `mcp add --scope user pictura -- <command> --mcp`

#### Scenario: A missing CLI is reported [reg_cli_missing]

- **WHEN** the tool is not found
- **THEN** the row says so and its button is disabled
