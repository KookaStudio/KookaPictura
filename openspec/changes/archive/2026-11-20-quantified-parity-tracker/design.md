# Design

## Context

See `proposal.md` for the motivation and `specs/meta/parity-tracker/spec.md`
for the requirements. The relevant current state:

- `ROADMAP.md` is an index of shipped/open work; `docs/dev/doc-code-parity-gaps.md`
  is the canonical gap register; `docs/TRACEABILITY.md` records spec existence,
  not implementation. None can be the coverage denominator.
- The CS6 contract and its Help-derived menu, tool, panel, and format lists are
  already in `docs/` (notably `docs/02-ui-ux/menus.md`, `docs/03-tools/`,
  `docs/02-ui-ux/panels/`, `docs/04-image-ops/`, `docs/06-filters/`,
  `docs/10-workflow-io/`). The tool catalogue has 71 CS6 slots, of which 61 are
  built and the ten gaps are the Extended 3D object and camera tools.
- No installed Photoshop is available on Linux. The CC baseline must be built
  from Adobe's public release notes and help pages, so the tracker measures
  feature coverage, never verified behavioral or pixel parity.
- `scripts/verify-fast.sh` has a docs-only fast path and a parallel check batch.
  The new checker follows the stdlib-only `--self-check` fixture convention used
  by checks such as `check-test-plan.py`; not every Python check uses that
  convention. `check-test-plan.py` accepts script coverage as
  `<script>::<fn>` when the function exists.
- A `docs/` change fails `scripts/guard.sh` unless the commit carries
  `TASK-ALLOWS-DOCS`.

## Goals / Non-Goals

**Goals:**

- One Markdown inventory that produces CS6 Standard, CS6 Extended, and CC 28.0
  coverage from the same rows.
- A reproducible counting rule and a checker that makes arithmetic drift a gate
  failure.
- A row format that keeps implementation evidence, verification, fidelity, and
  tracking links separate and auditable.
- A baseline construction that explains why the CS6 and CC denominators differ.

**Non-Goals:**

- Behavioral or pixel parity with Photoshop; no such claim is made.
- A weighted readiness score, audience tiers, dashboard, database, or generated
  website.
- A replacement for `docs/dev/doc-code-parity-gaps.md` or `ROADMAP.md`.
- Re-auditing every feature in this change. Rows without reconciled evidence are
  `Unknown` and improve in later sweeps.
- Runtime product behavior; this is a documentation and tooling change.

## Decisions

### One row set with per-baseline applicability

A row describes one named capability and carries three applicability cells
(CS6 Standard, CS6 Extended, CC 28.0) plus one Pictura status. The CS6 Standard,
CS6 Extended, and CC percentages are therefore three aggregates over the same
rows, not separate documents that can drift apart. A capability that CC
broadened or renamed gets a CC-only row when the user-facing capability is
materially new; otherwise it keeps one row and notes the rename.

Alternative rejected: three separate tables. It duplicates evidence and lets the
same feature hold different statuses in each table.

### Frozen baselines and a derived CC inventory

- CS6 Standard and Extended are frozen at Photoshop CS6 13.0 (2012). Standard is
  the non-Extended set; Extended adds 3D, measurement, and DICOM. Video/Timeline
  stays in Standard because CS6 moved it there from CS5 Extended.
- The project owner chose Photoshop 28.0 as the CC baseline, the November 2026
  release, frozen at the assessment date. Adobe's release notes name it
  "November 2026 (version 28.0)" and the 2026 LTS is 27.11.1; the baseline
  section cites that release note. It is derived explicitly:
  `CC = CS6 inventory − Adobe-retired features + Adobe-added capabilities`.
  Legacy 3D is the retirement example; Artboards is the CC-only example.
- The CS6 inventory comes from the CS6 contract plus Adobe CS6 Help-derived
  lists. The CC delta comes from Adobe release notes and help, not from extending
  the CS6 list by inference.
- A numbered Sources section holds the Adobe references. A CC-only or retired
  row's Notes carries `src:<n>`, and every `N/A` cell has a reason token:
  `edition:` for CS6 Standard's Extended-only cells, `post-CS6:` for CS6 cells of
  a CC-only row, and `retired:` for a CC cell whose feature Adobe removed.
- When Adobe sources disagree, the feature stays in the denominator: the
  applicability is `Y`, the status is `Unknown`, both citations and a
  `conflict:<src>` Notes token are recorded, and a disputed cell is never `N/A`.
  It is never resolved in Pictura's favour.

Alternative rejected: infer CC by adding the commonly known AI features to CS6.
That would make the "target" denominator a guess.

### Granularity defined by a named source list per category

The category names the list that defines its rows:

| Category | Row unit |
|---|---|
| Files & interchange | supported formats plus named File-menu, metadata, print, and web-export capabilities |
| Layers & compositing | named Layer-menu and Layers-panel capabilities |
| Tools & painting | CS6 toolbox slots (71, including the ten Extended 3D tools) plus named brush-engine, brush-dynamics, and preset capabilities |
| Selections & masking | named Select-menu capabilities |
| Adjustments & filters | the `Image > Adjustments` list (23 rows; 25 commands when Auto Tone/Contrast/Color are counted separately) plus the Filter menu's submenus and stand-alone filters (Liquify, Oil Paint, Filter Gallery, Blur Gallery, Adaptive Wide Angle, Lens Correction, Vanishing Point, Camera Raw, Lighting Effects) |
| Type & vector | Type-menu and Character/Paragraph capabilities plus type on path, warp text, type styles, and font handling |
| Color & bit depth | Image > Mode, Color Settings, picker and swatch capabilities |
| Automation & extensibility | File > Automate, Scripts, Actions, and plug-in capabilities |
| Shell, panels & preferences | Window-menu panels, Preferences panes, workspaces, shortcuts, accessibility |

The `Image > Adjustments` list is the one documented at
`docs/02-ui-ux/menus.md:134-138`: 23 rows, or 25 commands when Auto
Tone/Contrast/Color are counted separately. The Filter menu's submenus and
stand-alone filters are the ones documented at
`docs/02-ui-ux/menus.md:242-279`.

Dialog controls and individual options are Notes on their parent row. A filter,
blend-mode, or layer-style family is one row whose Notes carries
`members:<n>/<n>`. `Evidence` stays implementation paths only. Menu-command and
toolbox-present counts are reported as diagnostics beside the headline, never
added to a denominator.

Alternative rejected: one row per menu command (700+). It inflates the inventory
with labels that can be stubs, and it contradicts the issue's prohibition on
counting a parent workflow plus every control.

### Row schema

Each category has one table with this fixed shape:

```text
| ID | Feature | S | E | CC | Status | Scope | Evidence | Check | Notes | Tracking |
```

- `ID`: stable `P-<CAT>-<nn>`; existing rows never renumber.
- `S`, `E`, `CC`: `Y` or `N/A`; the Notes cell carries the required reason token
  or `src:<n>`. A disputed presence is `Y` with an `Unknown` status, both
  citations, and a `conflict:<src>` Notes token; a disputed cell is never `N/A`.
- `Status`: `Complete`, `Partial`, `Missing`, or `Unknown`.
- `Scope`: `in-scope` or `non-goal`.
- `Evidence`: semicolon-separated repo-relative paths only. `—` is allowed only
  for a row with no implementation.
- `Check`: `automated:<test-ref>`, `manual:<reason>`, or `none`.
- `Notes`: fidelity ceilings, conflicts (`conflict:<src>`), member counts
  (`members:<n>/<n>`), reason tokens, and sources (`src:<n>`). It never carries
  implementation paths.
- `Tracking`: for an in-scope row, a gap ID (`WF-11`, `TOOL-14`), an issue
  (`#1406`), or a `follow-up:<text>` note; required for every non-Complete
  in-scope row. A row with
  `Scope: non-goal` cites the ROADMAP or feasibility non-goal entry (for
  example `ROADMAP.md:691` or
  `docs/00-overview/feasibility-and-non-goals.md:72`) and is not required to
  name a gap-register row or issue.

Complete means the capability exists at the row's scope. It does not mean the
result matches Photoshop. Partial, Missing, and Unknown all earn zero completion
credit. Unknown is the conservative state when the register and tree disagree or
the sources conflict.

### Counting and rounding

```text
coverage(B) = 100 × Complete(B) / (Complete + Partial + Missing + Unknown)(B)
```

`N/A` rows leave that baseline's denominator. The full-product score uses every
applicable row. The in-scope score uses only `in-scope` rows, names the excluded
non-goal rows, and shows its own denominator; it never replaces the full-product
number. Percentages are displayed with one decimal place, rounded half up, and
always with the exact integer numerator and denominator. A zero baseline or
in-scope denominator displays `—`, not `0.0%`, and the checker accepts it. The
checker recomputes from the rows and compares the displayed strings.

The headline table shows Complete/Partial/Missing/Unknown counts per baseline and
the in-scope rows show their excluded count. The category table shows
`Complete/Denominator (percentage)` per baseline. The checker proves that the
category complete counts and denominators sum to the headline full-product
counts.

### Markdown is the source of truth; a stdlib checker enforces it

`PARITY.md` remains the single maintained artifact. No TOML/CSV source and no
generator: the row tables are readable and editable directly, and the checker
parses them. This is intentionally smaller than PhotoCraft's
source-file-plus-`xtask` scorecard while keeping its audit discipline.

`scripts/check-parity.py`:

- parses the fixed tables, the examples table, the largest-gaps section, and the
  baseline-history table;
- validates unique IDs (stability is a maintenance rule, not a mechanical
  check), enum values, applicability tokens, N/A reason tokens, evidence-path
  existence, Complete-row evidence, non-Complete in-scope tracking links, gap
  IDs against `docs/dev/doc-code-parity-gaps.md`, issue numbers as `#<digits>`,
  `follow-up:<text>` notes, non-goal citations against the ROADMAP or feasibility
  entries, `src:<n>`/`members:<n>/<n>`/`conflict:<src>` tokens, and `automated:` test
  references against existing test references;
- rejects `N/A` with a `conflict:<src>` note;
- recomputes the headline, in-scope, and category totals and compares the
  displayed values exactly, accepting `—` for a zero denominator;
- verifies that README and ROADMAP link `PARITY.md` and that each example
  resolves an existing row ID and source, not merely that six headings exist;
- runs `--self-check` fixtures for arithmetic drift, duplicate IDs, missing
  evidence, an unexplained N/A, missing or unresolved tracking links, missing
  examples, partial zero credit, in-scope counting, zero denominator, baseline
  history, largest gaps, and conflict applicability;
- exits non-zero with a named failure, and is wired into both the docs-only fast
  path and the parallel `start_check` batch of `scripts/verify-fast.sh`, plus
  the guards workflow.

The checker stays under 800 lines by keeping parsing, validation, and fixtures
in clearly separated functions; `scripts/check-file-size.sh` only scans
`crates/`, so this is a design rule, not an enforced gate.

Alternative rejected: a TOML data file and a Markdown generator. It doubles the
artifacts and adds a build step for a repository that already treats Markdown as
the contract.

### Maintenance

The tracker records the assessment date and Pictura commit. A feature ship or
removal updates the affected rows and totals in the same pull request as the
ROADMAP change. A new Photoshop major starts a new frozen baseline rather than
silently editing the old one, and the tracker keeps a baseline-history table with
one row per assessment (date, Pictura commit, CS6 Standard/Extended counts, CC
version, CC counts) so the new baseline does not erase the previous assessment.
Stable row IDs make historical comparisons possible, but the checker validates
uniqueness, not historical stability.

### Largest gaps and baseline history

`PARITY.md` carries a short largest-remaining-gaps section naming the largest
full-product gaps, the categories or rows driving them, and their tracking
sources. It also carries the baseline-history table described under Maintenance,
so the headline numbers have a visible audit trail across assessments.

### Stale claims corrected in this change

The tracker's measurement example and its own credibility depend on these:

- `README.md`: remove the claim that Adaptive Wide Angle, Lens Correction, and
  Vanishing Point have no engine; `ROADMAP.md` records them as shipped.
- `ROADMAP.md:30`: correct the At-a-glance Filters row that repeats the same
  stale "three CS6 families have no engine" claim.
- `ROADMAP.md`: replace "of which 21 are built" with the measured 61 of 71 and
  name the ten Extended 3D tool gaps.
- `ROADMAP.md:457`: change the open item to the Measurement Log panel only and
  remove "and the measurement engine".
- `ROADMAP.md:695-696`: remove the measurement engine and its Measurement Log
  panel from the "Not planned" bullet, leaving DICOM as the non-goal.
- `docs/dev/doc-code-parity-gaps.md:107` `PAN-2`: narrow the gap to the
  Measurement Log panel; the measurement scale, presets, and exporters ship.
- `docs/dev/doc-code-parity-gaps.md` `WF-11`: correct the measurement-scale row
  that still reads MISSING.
- `docs/00-overview/feasibility-and-non-goals.md:74` and the Qt6 mapping at line
  118: narrow the non-goal to DICOM; the Measurement Log panel is an open,
  in-scope gap (#510), not a non-goal.
- Tracker rows: measurement scale/presets/exporters = `Complete` (Extended)
  with #635; Measurement Log panel + Record Measurements = `Missing`,
  `in-scope`, #510; DICOM = `Missing`, `non-goal`, Extended.
- Other stale register rows are marked Unknown in the tracker and corrected in
  follow-up sweeps rather than silently awarded credit.

Because these touch `docs/`, the commit carries `TASK-ALLOWS-DOCS`.

## Risks / Trade-offs

- **The denominator is curated, not omniscient.** The source lists are frozen in
  the tracker; a Photoshop feature outside them is missing from the count.
  Mitigation: each category names its source list, and new baselines add rows
  with new IDs rather than rewriting history.
- **Assessment effort is large.** Hundreds of rows need evidence. Mitigation:
  complete the measurable surface first (tools, panels, formats, adjustments,
  filter families) and leave the rest Unknown; the percentage is then a
  conservative floor, not a false ceiling.
- **Baseline drift.** Photoshop 28.0 will be superseded quickly. Mitigation: the
  version/date is explicit, and the next major triggers a new frozen baseline.
- **Adobe source conflicts.** Lighting Effects is one known conflict. Mitigation:
  keep the row in the denominator, mark the conflict, and never award Complete
  from a disputed source.
- **Markdown parsing fragility.** A hand edit can break a table. Mitigation:
  fixed column count per table, a checker with self-check fixtures, and a named
  error for each failure.
- **Optics.** The CC full-product number will look low because AI and cloud
  features are included. Mitigation: show full-product and in-scope side by side
  and label them; do not hide the exclusions.
- **Tracker rot.** A tracker that is not updated becomes a lie. Mitigation: the
  update rule, stable row IDs, the arithmetic gate, and a link from ROADMAP.

## Migration Plan

No runtime or data migration. The implementation lands as Markdown, one Python
checker, links, and documentation corrections. The checker runs in both the
docs-only fast path and the parallel `start_check` batch of `verify-fast.sh`; no
build is required. Rollback is a revert of the documentation and checker files.
After the change lands, archive it with
`openspec archive quantified-parity-tracker`.

## Open Questions

None. The owner-chosen CC baseline (Photoshop 28.0, the November 2026 release),
the capability-level granularity, and the stale-claim corrections are settled.
