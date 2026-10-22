## ADDED Requirements

### Requirement: Saves land through an unpredictable temp file

Native saves, raster and PDF exports, and recovery snapshots SHALL write a
sibling temp file named with 64 bits from the operating system's random
source, created only if no entry exists at that name. The temp file SHALL be
renamed over the target, and removed if any step fails. Encryption salts,
PDF file identifiers, and scratch names SHALL come from the operating
system's random source.

#### Scenario: A planted link is not followed [as_planted_link]

- **WHEN** a symlink to another file sits at `<path>.tmp` and the document is
  saved to `<path>`
- **THEN** the link's target is unchanged and `<path>` holds the new bytes

#### Scenario: The target is replaced cleanly [as_replace]

- **WHEN** an existing file is saved over
- **THEN** it holds the new bytes and no temp file is left, and two temp
  names drawn in a row differ
