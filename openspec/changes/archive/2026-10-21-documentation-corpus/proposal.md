# Proposal

## Why

The user-facing documentation still describes the project as it was before the
tool, export, and transform work landed.

- `ROADMAP.md` lists as planned a set of tools that ship: the eraser family,
  gradient and paint bucket, clone and pattern stamp, dodge/burn/sponge,
  blur/sharpen/smudge, the type tools, the healing family, the pen and path
  tools, the shape tools, magnetic lasso, rotate view, the slice tools, the
  colour sampler/ruler/note/count tools, perspective crop, Save As and Export,
  Save for Web, recent files, align and distribute, and clipping masks. It
  also claims Pictura Raw exposes only its Basic controls, when eight of its
  nine tabs are wired to a Rust renderer.
- `README.md` omits the shipped surface, its "Layers" bullet omits shape and
  type layers, and two of its claims overstate the code: "the full classic
  filter set" (three CS6 filter families have no engine) and "all the usual
  adjustment families" (several of them are partial, and the roadmap now
  names the control each one still lacks).
- 137 files under `docs/` carry the banner "No code exists in this repository",
  and `docs/README.md` adds "no code, no build files, and no dependencies" on
  top of "Documentation-only specification". The repository contains eleven Rust
  crates, a CMake/Corrosion build, and a shipping binary.
- `docs/dev/doc-code-parity-gaps.md` — the canonical register of what is
  specified but missing — is itself wrong in five places, so pointing the
  roadmap at it would carry the errors forward.

The gap register already exists: 160 gaps, each with a status, an evidence
`path:line`, and a tracking issue (#489–#651), filed under #488. This change
does not re-derive it. It re-points the user-facing documentation at it, and
adds the contract that keeps the two from drifting apart again.

## What Changes

- Re-cut `ROADMAP.md` into Shipped, Open, and Not planned. Every shipped
  feature is ticked and every remaining item is an open gap that links the
  GitHub issue tracking it, so the roadmap becomes a navigable index of the
  register instead of a stale phase list.
- Extend `README.md`'s feature list to the surface the app actually ships,
  give each bullet a path in the tree to check, and replace the two
  over-claims with the real counts.
- Replace the "no code exists" banner in 137 corpus files with one that is
  true, keeping each file's source-provenance sentence intact.
- Correct the corpus entry point and the product overview, which describe the
  repository as a corpus with no implementation.
- Correct the five wrong rows in `docs/dev/doc-code-parity-gaps.md`, so the
  register the roadmap points at is trustworthy.
- Add `scripts/check-doc-claims.py` and two `scripts/guard.sh` rules, so a
  banner that denies the implementation, or a roadmap item that names no
  tracking issue, fails the guards workflow.
- Add an `AGENTS.md` rule requiring a pull request that ships or removes a
  feature to update `ROADMAP.md` in the same change.

## Capabilities

### New Capabilities

- `meta/documentation-corpus`: the README, the roadmap, and the specification
  corpus describe the application that ships; every claim is checkable, every
  open item names its tracking issue, and the gap register is the single
  index of what is still missing.

### Modified Capabilities

- None. No behavioural requirement of the application changes.

## Impact

- `ROADMAP.md`, `README.md` (repository root; no `docs/` marker needed).
- `AGENTS.md`: one new rule.
- `docs/`: 141 files — 137 banners, `docs/README.md`,
  `docs/00-overview/product-overview.md`,
  `docs/00-overview/feasibility-and-non-goals.md`,
  `docs/dev/doc-code-parity-gaps.md`, and `docs/dev/STATE.md`. Carries
  `TASK-ALLOWS-DOCS`.
- `scripts/check-doc-claims.py` (new) and `scripts/guard.sh` (rules 12 and 13).
- `openspec/specs/meta/documentation-corpus/spec.md` (new, via archive).
- No product code, no engine crate, no dependency, no behaviour change.
