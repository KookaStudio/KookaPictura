# Proposal

## Why

Issue #1425: Kooka Pictura has no quantified, evidence-backed view of how much of
Photoshop CS6 (the project floor) and the current Photoshop CC desktop release
(the parity goal) it actually covers. The roadmap, the gap register, and
`docs/TRACEABILITY.md` answer different questions — what ships, what is missing
against the corpus, and whether a spec exists — so none of them can serve as a
feature-coverage denominator. Without a frozen inventory and explicit counting
rules, any "X% complete" statement is unverifiable, and stale roadmap or register
claims silently inflate it.

## What Changes

- Add `PARITY.md` at the repository root: one capability inventory assessed
  against three frozen baselines — Photoshop CS6 Standard, Photoshop CS6
  Extended, and Photoshop CC 28.0 (the November 2026 release). It carries
  headline full-product and in-scope percentages with their numerators and
  denominators, category breakdowns, unique feature IDs, per-row status and
  baseline applicability, implementation evidence, separate verification
  evidence, fidelity/limitation notes, a baseline-history table, the
  largest-remaining-gaps explanation, and the issue or gap-register row tracking
  unfinished work.
- Freeze the CS6 inventories from the CS6 contract in `docs/` and the
  CS6 Help-derived menu, tool, panel, and format lists. The project owner chose
  Photoshop 28.0 as the CC baseline, the November 2026 release; Adobe's release
  notes name it "November 2026 (version 28.0)" and the 2026 LTS is 27.11.1,
  cited in the tracker's baseline section. Freeze the CC inventory from those
  public notes and help, derived as `CS6 − retired + additions`; legacy 3D is
  the retired example, Artboards a CC-only example.
- Add `scripts/check-parity.py`: a stdlib-only checker that parses the
  Markdown tables, recomputes every baseline and category total, and validates
  unique IDs, baseline applicability, statuses, N/A reasons, evidence paths,
  gap-register IDs, issue numbers, `src:<n>`/`members:<n>/<n>`/`conflict:<src>`
  tokens, and automated test references. It ships a `--self-check` fixture suite
  and runs in both the docs-only fast path and the parallel check batch of
  `scripts/verify-fast.sh`, and in the guards CI workflow.
- Link the tracker from `README.md` and `ROADMAP.md`; the gap register links to
  it as the aggregate view and remains the canonical gap register.
- Correct the stale claims the headline depends on: the README statement that
  three filter families have no engine and the matching `ROADMAP.md` At-a-glance
  Filters row, the ROADMAP tool-count sentence, and the measurement claims in
  `ROADMAP.md`, `docs/dev/doc-code-parity-gaps.md`, and
  `docs/00-overview/feasibility-and-non-goals.md`. Reconcile the affected
  gap-register rows, or mark their tracker rows Unknown until corrected.
- Record the maintenance rule: a feature ship or removal updates the affected
  tracker rows and totals in the same pull request that updates `ROADMAP.md`,
  and existing row IDs stay stable.

## Capabilities

### New Capabilities

- `meta/parity-tracker`: the maintained comparison contract — three frozen
  baselines, one capability inventory at a documented granularity, status and
  applicability semantics, full-product versus in-scope counting, evidence and
  verification separation, baseline-conflict handling, baseline history, the
  worked examples, the largest-remaining-gaps explanation, the update rule, and
  the arithmetic check.

### Modified Capabilities

None. The README and ROADMAP corrections enforce the existing
`meta/documentation-corpus` requirements; no requirement text changes there.

## Impact

- New root `PARITY.md`; new `scripts/check-parity.py`.
- Links added in `README.md`, `ROADMAP.md`, and
  `docs/dev/doc-code-parity-gaps.md`; corrections in `ROADMAP.md`,
  `docs/dev/doc-code-parity-gaps.md`, and
  `docs/00-overview/feasibility-and-non-goals.md` (a `docs/` change, so the
  commit carries `TASK-ALLOWS-DOCS`).
- `scripts/verify-fast.sh` and the guards workflow gain the parity check; the
  `Coverage` rule in `openspec/config.yaml` gains `<script>::<fn>` support.
- No product code, API, UI behavior, or dependency changes. The checker uses the
  Python standard library only.
