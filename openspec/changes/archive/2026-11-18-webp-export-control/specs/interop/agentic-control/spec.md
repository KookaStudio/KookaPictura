# agentic-control delta

## ADDED Requirements

### Requirement: The control server exports WebP

The control server SHALL provide `export_webp`. It SHALL require `path`, accept
an optional `quality` from 0 through 100 defaulting to 90, and accept an
optional `lossless` boolean defaulting to false. With `lossless` true it SHALL
write a VP8L stream; otherwise it SHALL write a lossy VP8 stream. It SHALL write
the flattened composite atomically, SHALL NOT change the document's path,
modified state, or history, and SHALL be gated like a save: a restricted session
without the Save grant SHALL answer `refused` and write nothing.

#### Scenario: Lossy is the default [ce_export_webp]

- **WHEN** a client calls `export_webp` with only a path
- **THEN** the file is a WebP container with a VP8 chunk and no VP8L chunk

#### Scenario: Lossless writes VP8L [ce_export_webp_lossless]

- **WHEN** a client calls `export_webp` with `lossless` true
- **THEN** the file is a WebP container with a VP8L chunk and no VP8 chunk

#### Scenario: Export leaves the document alone [ce_export_webp_state]

- **WHEN** a client exports a dirty document as WebP
- **THEN** the document's path, modified state, and history count are unchanged

#### Scenario: Export is gated like a save [ce_export_webp_gate]

- **WHEN** a restricted session without the Save grant calls `export_webp`
- **THEN** the request is refused and no file is written
