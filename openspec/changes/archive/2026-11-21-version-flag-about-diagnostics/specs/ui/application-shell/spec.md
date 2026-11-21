# Spec Delta

## ADDED Requirements

### Requirement: The shell reports its application version on the command line

Running `pictura --version` SHALL write one newline-terminated line of the
form `Kooka Pictura <version>` to stdout and exit with status 0. The version
SHALL match the version shown in About. The command MUST work without a display
and MUST NOT start the GUI, probe the GPU, check for updates, or modify session
state.

On Windows the packaged executable SHALL provide this output when stdout is
piped or redirected and SHALL retain its GUI subsystem for normal launches.

#### Scenario: A script reads the version without a display [shell_version_flag]

- **WHEN** a script runs `pictura --version` in an environment without a usable
  Qt platform plugin, using piped or redirected stdout on Windows
- **THEN** the process exits with status 0 and stdout contains exactly one
  line, `Kooka Pictura <version>`, with the build's application version
- **AND** stderr is empty, the session state is untouched, and no update
  request is made
