## ADDED Requirements

### Requirement: Control document paths are validated

The control server's `document` method SHALL refuse with `invalid_param`,
without opening or writing anything, an `open`, `save_as`, or `save` path
that is not absolute, contains a character below U+0020 or U+007F, or has a
`..` component. A `save_as`, or a `save` with a path, SHALL also refuse a
path whose final component is a symbolic link, and a `save` with no path
SHALL refuse the same way when the active document's path is one. Any other
absolute path SHALL be accepted, so the server SHALL NOT confine paths to a
root directory.

#### Scenario: Ambiguous paths are refused [ac_path_refused]

- **WHEN** a client opens a relative path, a path holding a newline, and an
  absolute path with a `..` component
- **THEN** each is answered `invalid_param` and no document opens

#### Scenario: A save over a symlink is refused [ac_path_save_link]

- **WHEN** a client calls `save_as` on a symlink that points at another file
- **THEN** the response is `invalid_param` and the link's target is unchanged

#### Scenario: A pathless save through an opened symlink is refused [ac_path_save_active_link]

- **WHEN** a client opens a symlink to a document, then calls `save` with no path
- **THEN** the response is `invalid_param` and the link's target is unchanged

#### Scenario: A plain absolute path still works [ac_path_accepted]

- **WHEN** a client calls `save_as` on a new absolute path in a temp directory,
  then `open` on it
- **THEN** both succeed and the file exists
