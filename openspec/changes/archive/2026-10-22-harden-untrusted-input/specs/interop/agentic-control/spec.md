## ADDED Requirements

### Requirement: Control requests carry a session token

On listening, the control server SHALL write a fresh random session token
(64 hex digits) to `<socket>.token`, readable and writable by the owner only,
created fresh without following an existing link, and removed on shutdown.
Every socket request SHALL carry the token as its `token` member; a
well-formed request without it, or with any other value, SHALL be answered
`unauthorized` (echoing its `id`) and not dispatched.

#### Scenario: A request without the token is refused [ac_token_required]

- **WHEN** a client sends `status` with no token, a wrong token of the same
  length, or a short one, and then with the session token
- **THEN** the first three are answered `unauthorized` with their `id`, and
  the last succeeds

#### Scenario: The token file is private [ac_token_file]

- **WHEN** the server listens where a symlink to another file was planted at
  the token path, and later shuts down
- **THEN** the token file is a regular owner-only file holding the token, the
  link's target is untouched, a second server's token differs, and shutdown
  removes the file
