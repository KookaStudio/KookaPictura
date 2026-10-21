# Spec Delta

## Purpose

Binds the user-facing documentation to the application that ships: the README
and the roadmap describe only what the code backs, every claim is checkable
against a path in the tree, and every remaining gap is an open item that names
the issue tracking it.

## ADDED Requirements

### Requirement: The roadmap claims only what ships

`ROADMAP.md` SHALL mark a feature as shipped only when the tree implements it,
and every shipped feature SHALL be marked as shipped. Each shipped entry SHALL
name a path in the tree that backs the claim. The roadmap SHALL NOT use a
phase number as a substitute for a status.

#### Scenario: A shipped entry names a path that exists [dc_roadmap_shipped]

- **WHEN** a reader follows a shipped entry in `ROADMAP.md` to the path it names
- **THEN** the path exists in the tree and the code at that path implements the
  claimed feature

#### Scenario: An open entry names the issue that tracks it [dc_roadmap_open]

- **WHEN** a reader reads an unticked entry in `ROADMAP.md`
- **THEN** the entry names at least one GitHub issue, and that issue is tracked
  in the gap register at `docs/dev/doc-code-parity-gaps.md`

### Requirement: The README describes the application that ships

Every feature bullet in `README.md` SHALL name a path in the tree that backs
the claim. The README SHALL NOT claim a feature the tree does not implement,
and where a family is only partly implemented the bullet SHALL say so rather
than name the family whole.

#### Scenario: A feature bullet names a path that exists [dc_readme_bullet]

- **WHEN** a reader follows a feature bullet in `README.md` to the path it names
- **THEN** the path exists in the tree and the code at that path implements the
  claimed feature

### Requirement: The specification corpus does not deny the implementation

No file under `docs/` SHALL assert that the repository contains no source code,
no build files, or no dependencies, while the repository holds the
implementation the corpus specifies. A corpus file's status banner SHALL name
`docs/dev/doc-code-parity-gaps.md` as the place that records what shipped and
what did not. The corpus entry point SHALL describe itself as the behavioural
contract for the project and SHALL name where the implementation lives.

#### Scenario: A corpus file's banner points at the gap register [dc_corpus_banner]

- **WHEN** a reader reads the status banner of a file under `docs/`
- **THEN** the banner does not claim the repository has no code, and it names
  `docs/dev/doc-code-parity-gaps.md`

#### Scenario: The corpus entry point names the implementation [dc_corpus_entry]

- **WHEN** a reader opens `docs/README.md`
- **THEN** it describes the corpus as the behavioural contract for the project
  and names the directory holding the implementation, rather than claiming the
  repository has no code, no build files, and no dependencies

### Requirement: The gap register is the single index of what is missing

`docs/dev/doc-code-parity-gaps.md` SHALL remain the canonical register of the
gaps between the corpus and the code, carrying for each gap a status, an
evidence path, and the GitHub issue tracking it. A row whose evidence the tree
contradicts SHALL be corrected in the register rather than left to mislead the
roadmap that points at it.

#### Scenario: A roadmap item resolves to a register row [dc_register_index]

- **WHEN** a reader takes an unticked entry in `ROADMAP.md` and follows its
  issue into `docs/dev/doc-code-parity-gaps.md`
- **THEN** the issue appears in the register's Issue column, and the status the
  register records for that gap agrees with what the roadmap says about it

### Requirement: The roadmap and the corpus agree on what is not planned

A feature that the tree implements SHALL NOT be listed as a non-goal. A feature
the tree does not implement and that is not tracked by an issue SHALL be
listed as a non-goal with the reason, and a feature the tree does not
implement that is tracked by an issue SHALL be listed as open rather than as a
non-goal.

#### Scenario: A shipped feature is not a non-goal [dc_nongoal_shipped]

- **WHEN** a reader takes a feature listed under "Not planned" in `ROADMAP.md`
  and searches the tree for its implementation
- **THEN** no implementation contradicts the non-goal

#### Scenario: An absent non-goal is deliberate [dc_nongoal_absent]

- **WHEN** a reader takes a feature listed under "Not planned" in `ROADMAP.md`
  that the gap register records as a confirmed non-goal
- **THEN** the register agrees, and the feature's absence is deliberate rather
  than a gap that no issue tracks
