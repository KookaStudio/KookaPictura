## ADDED Requirements

### Requirement: Canvas orientation runs by id

`run_command` SHALL list `image.rotate90cw`, `image.rotate90ccw`,
`image.rotate180`, `image.flipHorizontal` and `image.flipVertical` with no
parameter keys, and SHALL run each through its replay path. A command its
memory gate refuses SHALL be `refused` and SHALL leave the document unchanged.

#### Scenario: A quarter turn runs by id [ce_run_orient]

- **WHEN** a client runs `image.rotate90cw` on a 60 × 40 document
- **THEN** the document is 40 × 60

#### Scenario: An orientation command takes no keys [ce_run_orient_key]

- **WHEN** a client passes `angle` to `image.flipHorizontal`
- **THEN** the response is `invalid_param`
