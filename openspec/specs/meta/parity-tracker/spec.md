# meta/parity-tracker Specification

## Purpose
Quantifies Kooka Pictura's feature coverage against a frozen Photoshop CS6 floor
and the current Photoshop CC desktop target from one evidence-backed capability
inventory, with transparent counting that a checked Markdown tracker can
reproduce.

## Requirements

### Requirement: The tracker is the aggregate comparison, not a second backlog

The repository SHALL maintain `PARITY.md` at the root as the aggregate comparison
of Pictura against its Photoshop baselines. `README.md` and `ROADMAP.md` SHALL
link to it. `docs/dev/doc-code-parity-gaps.md` SHALL remain the canonical gap
register: every in-scope tracker row that is not Complete SHALL name the
gap-register row, a GitHub issue, or a clear `follow-up:` note that tracks the
remaining work, and the tracker SHALL NOT introduce a competing gap register. A tracker row with
`Scope: non-goal` SHALL cite the ROADMAP or feasibility non-goal entry and is
not required to name a gap-register row or GitHub issue.

#### Scenario: README and ROADMAP link the tracker [pt_linked]

- **WHEN** a reader opens `README.md` or `ROADMAP.md`
- **THEN** each links to `PARITY.md`

#### Scenario: An incomplete row names its tracking source [pt_gap_register]

- **WHEN** an in-scope tracker row has status Partial, Missing, or Unknown
- **THEN** the row names an existing gap-register row, a GitHub issue, or a
  clear `follow-up:` note that tracks the remaining work

#### Scenario: A non-goal row cites its non-goal source [pt_nongoal_tracking]

- **WHEN** a tracker row has `Scope: non-goal`
- **THEN** it cites the ROADMAP or feasibility non-goal entry and is not required
  to name a gap-register row or GitHub issue

#### Scenario: The gap register stays canonical [pt_no_second_register]

- **WHEN** a reader looks for the authoritative status and evidence of a gap
- **THEN** `docs/dev/doc-code-parity-gaps.md` remains that register, and the
  tracker's incomplete in-scope rows point to it rather than duplicating it

### Requirement: Three frozen, sourced baselines

The tracker SHALL assess three baselines: Photoshop CS6 Standard, Photoshop CS6
Extended, and the Photoshop CC desktop release named by the assessment. It SHALL
record each baseline's exact version or edition and the assessment date.
CS6 Standard SHALL exclude the Extended-only feature groups; CS6 Extended SHALL
include them; video and the Timeline SHALL remain in CS6 Standard. The CC
baseline SHALL be derived from the CS6 inventory by removing features Adobe
retired and adding capabilities Adobe introduced and still ships in the named
release. Every `N/A` applicability SHALL have a reason, and every retirement or
CC-only addition SHALL cite an Adobe source. The CC inventory SHALL be researched
from Adobe's public release notes and help, not inferred from the CS6 list. The
baseline section SHALL cite the Adobe source that names the CC release.

#### Scenario: The baselines name their versions and date [pt_baselines]

- **WHEN** a reader opens the tracker's baseline section
- **THEN** each of CS6 Standard, CS6 Extended, and CC names its version or
  edition and the assessment date, the tracker records the assessed Pictura
  commit, and the CC baseline cites the Adobe source that names the release

#### Scenario: CS6 Standard excludes only Extended-only groups [pt_cs6_editions]

- **WHEN** the tracker assesses an Extended-only feature such as 3D,
  measurement, or DICOM against CS6 Standard
- **THEN** that baseline's applicability is N/A and the edition rule is the
  stated reason

#### Scenario: A retirement is N/A in CC with a source [pt_cc_retired]

- **WHEN** Adobe retired a CS6 feature before the named CC release
- **THEN** the CC applicability is N/A and the row cites the retirement source

#### Scenario: A CC-only capability is N/A in CS6 [pt_cc_added]

- **WHEN** a capability exists in the named CC release but not in CS6
- **THEN** both CS6 applicability cells are N/A and the row cites the Adobe
  source that establishes the CC capability

#### Scenario: The CC inventory is researched [pt_cc_researched]

- **WHEN** a reader checks a CC-only or retired row
- **THEN** the Notes cell cites an Adobe release-note or help source for the
  named CC release, not an inference from the CS6 inventory

### Requirement: The tracker preserves baseline history

The tracker SHALL keep a baseline-history table with one row per assessment,
recording the assessment date, the assessed Pictura commit, the CS6 Standard and
CS6 Extended counts, the CC version, and the CC counts. A new Photoshop major
SHALL add a row rather than erase the previous assessment.

#### Scenario: A new assessment appends history [pt_baseline_history]

- **WHEN** a reader opens the baseline-history table after a new assessment
- **THEN** it contains one row per assessment with the date, Pictura commit,
  CS6 Standard and CS6 Extended counts, CC version, and CC counts, and the
  previous row is still present

### Requirement: One inventory at a documented granularity

The tracker SHALL use one capability inventory at a documented granularity. Each
category SHALL name its source list, and each row SHALL correspond to one entry
in that list. The category source lists SHALL cover:

- Files & interchange: supported formats plus named File-menu, metadata, print,
  and web-export capabilities.
- Layers & compositing: named Layer-menu and Layers-panel capabilities.
- Tools & painting: CS6 toolbox slots (71, including the ten Extended 3D tools)
  plus named brush-engine, brush-dynamics, and preset capabilities.
- Selections & masking: named Select-menu capabilities.
- Adjustments & filters: the `Image > Adjustments` list (23 rows; 25 commands
  when Auto Tone/Contrast/Color are counted separately) plus the Filter menu's
  submenus and stand-alone filters, per `docs/02-ui-ux/menus.md:242-279`
  (Liquify, Oil Paint, Filter Gallery, Blur Gallery, Adaptive Wide Angle, Lens
  Correction, Vanishing Point, Camera Raw, Lighting Effects).
- Type & vector: Type-menu and Character/Paragraph capabilities plus type on
  path, warp text, type styles, and font handling.
- Color & bit depth: `Image > Mode`, Color Settings, picker and swatch
  capabilities.
- Automation & extensibility: File > Automate, Scripts, Actions, and plug-in
  capabilities.
- Shell, panels & preferences: Window-menu panels, Preferences panes,
  workspaces, shortcuts, accessibility.

Dialog controls and individual options SHALL NOT be rows. When a family is a
single row, its Notes SHALL state the implemented and total member count using
`members:<n>/<n>`. Menu-command or toolbox presence counts SHALL be reported as
diagnostics and SHALL NOT be added to the capability denominator.

#### Scenario: Rows follow the category's source list [pt_granularity]

- **WHEN** a reader inspects a category's rows
- **THEN** each row corresponds to an entry in the source list the category names,
  the category covers its expanded source units (including stand-alone filters
  and the brush and type capabilities), and no row is a dialog control or an
  individual option

#### Scenario: A family row carries its member counts [pt_family_members]

- **WHEN** filters, blend modes, or layer-style effects are represented by one
  family row
- **THEN** the row's Notes cell states how many members are present and how many
  the baseline has using `members:<n>/<n>`

#### Scenario: Surface counts stay out of the headline [pt_surface_counts]

- **WHEN** the tracker reports a menu-command or toolbox-present count
- **THEN** it is labelled a diagnostic and does not change a baseline's
  capability denominator

### Requirement: Transparent counting with no invented credit

For each baseline the tracker SHALL report
`100 × Complete / (Complete + Partial + Missing + Unknown)`, where an `N/A`
applicability is excluded from that baseline's denominator and Partial, Missing,
and Unknown receive no completion credit. The tracker SHALL NOT introduce
partial-credit weights or estimated weights. It SHALL show a full-product score
and, separately, an in-scope score that excludes only rows declared to be
non-goals, names those exclusions, and shows its own denominator; the in-scope
score SHALL NOT substitute for the full-product score. Headline and category
totals SHALL reconcile exactly with the rows for every baseline, and every
percentage SHALL show its numerator and denominator and the documented rounding
rule. A zero baseline or in-scope denominator SHALL display `—`, not `0.0%`.

#### Scenario: Partial earns no completion credit [pt_partial_zero]

- **WHEN** a row is Partial for a baseline
- **THEN** it is counted in that baseline's denominator and not in its Complete
  count

#### Scenario: The in-scope score is separate and visible [pt_in_scope]

- **WHEN** the tracker shows an in-scope percentage
- **THEN** it names the excluded non-goal rows and shows the in-scope numerator
  and denominator, and the full-product percentage remains visible

#### Scenario: Headline and category totals reconcile [pt_reconcile]

- **WHEN** the tracker's headline or category totals are compared with its rows
- **THEN** they match exactly for every baseline

#### Scenario: Percentages show a reproducible rounding [pt_rounding]

- **WHEN** the tracker displays a percentage
- **THEN** it shows the exact numerator and denominator and applies the one
  documented rounding rule

#### Scenario: A zero denominator displays as a dash [pt_zero_denominator]

- **WHEN** a baseline or the in-scope score has no applicable rows
- **THEN** the tracker displays `—` rather than `0.0%`, and the checker accepts it

### Requirement: Implementation evidence and verification are distinct

Every Complete row SHALL name an implementation path that exists in the tree. A
row's verification SHALL be recorded separately from its status as an automated
check, a manual check with a reason, or none. The tracker SHALL record fidelity
limitations such as approximate or modelled behavior and Pictura-only formats.
It SHALL state that coverage is not behavioral or pixel parity with Photoshop,
and SHALL NOT claim verified parity without a check that demonstrates it.

#### Scenario: A Complete row names existing evidence [pt_complete_evidence]

- **WHEN** a row is Complete
- **THEN** it names at least one implementation path, and that path exists in the
  tree

#### Scenario: Verification is recorded separately [pt_verification]

- **WHEN** a row claims verification
- **THEN** its check field names an automated test reference that resolves to an
  existing test or a manual procedure with a reason

#### Scenario: The tracker does not claim Photoshop parity [pt_no_parity_claim]

- **WHEN** the tracker describes its headline numbers
- **THEN** it states that they measure feature coverage, not behavioral or pixel
  parity, and no row claims verified parity without a check

### Requirement: Unknown and conflicting evidence stay visible

The tracker SHALL use Unknown when the evidence is insufficient or the sources
conflict. When a gap-register row and the tree disagree, the affected row SHALL
be Unknown until the register is corrected. When Adobe sources disagree about a
feature's presence in a baseline, the feature SHALL stay in that baseline's
denominator: the applicability SHALL be `Y`, the status SHALL be `Unknown`, both
citations SHALL be recorded, and the row SHALL carry a `conflict:<src>` Notes
token. A disputed cell SHALL NOT be `N/A`, and the conflict SHALL NOT be resolved
in Pictura's favour.

#### Scenario: A register-versus-tree conflict is Unknown [pt_conflict_unknown]

- **WHEN** the gap register says a feature is missing while the tree implements
  it, or the reverse
- **THEN** the tracker row is Unknown and cites both sources until the register
  is corrected

#### Scenario: A disputed baseline presence stays counted [pt_baseline_dispute]

- **WHEN** Adobe sources disagree about whether a feature exists in the baseline
- **THEN** that applicability is `Y`, the status is `Unknown`, both citations and
  a `conflict:<src>` Notes token are present, the feature remains in the
  denominator, and the cell is never `N/A`

### Requirement: The tracker carries the required worked examples

The tracker SHALL include one sourced example of each of: a complete workflow, a
partial implementation, an unknown assessment, an intentional non-goal, a
CC-only capability, and a capability Adobe retired before the CC baseline. Each
example SHALL name its tracker row and its source.

#### Scenario: All six examples are present [pt_examples]

- **WHEN** a reader opens the tracker's worked-examples section
- **THEN** it contains a complete workflow, a partial implementation, an unknown
  assessment, an intentional non-goal, a CC-only capability, and a retired
  capability, each naming its row and source

### Requirement: The tracker explains the largest remaining gaps

The tracker SHALL include a largest-remaining-gaps section naming the largest
full-product gaps, the categories or rows driving them, and their tracking
sources.

#### Scenario: The largest gaps are explained [pt_largest_gaps]

- **WHEN** a reader opens the largest-remaining-gaps section
- **THEN** it names the largest full-product gaps, the categories or rows driving
  them, and their tracking sources

### Requirement: Maintenance and arithmetic check

A pull request that ships or removes a feature SHALL update the affected tracker
rows and totals in the same pull request that updates `ROADMAP.md`. Row IDs SHALL
be stable: an existing row keeps its ID, and a new capability receives a new ID
rather than renumbering the inventory. A new Photoshop major SHALL trigger a new
frozen baseline and assessment rather than silent edits to the old one. The
repository SHALL provide `scripts/check-parity.py`, a standard-library-only
checker that parses the Markdown tables, recomputes every baseline and category
total, and validates unique IDs, applicability and status values, N/A reasons,
evidence paths, `src:<n>`/`members:<n>/<n>`/`conflict:<src>` tokens, in-scope
tracking links resolved to gap-register rows, issues, or `follow-up:` notes, and
automated test references. The checker SHALL provide a self-check fixture suite and SHALL run in
both the docs-only fast path and the parallel check batch of
`scripts/verify-fast.sh`, and in the guards CI workflow.

#### Scenario: The update rule is stated [pt_update_rule]

- **WHEN** a feature ships or is removed
- **THEN** the tracker states that the affected rows and totals change in the
  same pull request as the roadmap update

#### Scenario: Arithmetic drift fails the check [pt_drift]

- **WHEN** a headline or category total in `PARITY.md` does not match the rows
- **THEN** `scripts/check-parity.py` exits non-zero and names the mismatch

#### Scenario: Duplicate IDs fail the check [pt_duplicate_id]

- **WHEN** two rows share a tracker ID
- **THEN** `scripts/check-parity.py` exits non-zero

#### Scenario: A Complete row without evidence fails the check [pt_missing_evidence]

- **WHEN** a Complete row has no existing evidence path
- **THEN** `scripts/check-parity.py` exits non-zero

#### Scenario: An unexplained N/A fails the check [pt_na_reason]

- **WHEN** a baseline applicability is N/A and the row gives no reason or source
- **THEN** `scripts/check-parity.py` exits non-zero

#### Scenario: The check is wired into the gates [pt_check_wired]

- **WHEN** `scripts/verify-fast.sh` runs its docs-only fast path or its parallel
  check batch, or the guards CI workflow runs
- **THEN** the path runs `scripts/check-parity.py --self-check` and
  `scripts/check-parity.py`, and a failure fails the gate
