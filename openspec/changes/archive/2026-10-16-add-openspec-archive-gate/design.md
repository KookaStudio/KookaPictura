# Design

## Context

See proposal.md — Why. The archive check is a directory-existence test over
`openspec/changes/`. The existing shell checks establish the shape: `guard.sh`
runs as the workflow's non-goal guard, while `check-selftest-budget.sh` and
`check-worktree-hook.sh` are standalone scripts wired into both the guards
workflow and `scripts/verify-fast.sh`. `verify-fast.sh` has two paths — a
docs-only fast path and the full gate — and an `openspec/`-only diff takes the
docs-only path. The backllog spans two requirements that two changes each modify
(`MODIFIED` replaces the whole requirement by name), and archiving a change that
creates a new capability leaves a `TBD` Purpose placeholder that `--strict`
rejects as a warning.

## Goals / Non-Goals

**Goals:**
- Fail any pull request, push, or release pull request that leaves a non-archived
  change under `openspec/changes/`.
- Make the check non-vacuous and local-runnable.
- Land the backlog migration with the merged specs matching the shipped code.

**Non-Goals:**
- Changing what a change proposal contains, or any Rust/C++ behavior.
- Adding an explicit archive-check job to `release-please.yml` (covered by the
  token; see D4).
- Re-archiving or rewriting already-archived history.

## Decisions

**D1 — A standalone `scripts/check-openspec-archived.sh`, not a new clause in
`guard.sh`.** `guard.sh` is the *non-goal artifact* guard with a documented rule
list; the budget check is already a separate script wired the same way. Mirroring
`check-selftest-budget.sh` keeps each guard's intent legible and its wiring
uniform. Alternative rejected: a clause in `guard.sh` — conflates two unrelated
policy checks in one file.

**D2 — The check takes an optional directory argument and ships a `--self-test`.** A
hardcoded repo path cannot be exercised against a fixture, so the check would be
untestable. The argument defaults to `openspec/changes`; `--self-test` builds a
`mktemp` sandbox with a stray change directory and an `archive`-only directory,
asserts fail-then-pass, and cleans up (shape from `check-worktree-hook.sh`).
Alternative rejected: no self-test on a "trivial" check — this one gates every
future pull request, so a silent regression (e.g. a `find` that stops matching)
must fail loudly.

**D3 — Wire the check into both `verify-fast.sh` paths.** An `openspec/`-only diff
takes the docs-only fast path, which is exactly the case the gate exists to
catch. Running it only in the full gate would let an added change pass locally.
The guards workflow runs it unconditionally regardless.

**D4 — Rely on `RELEASE_PLEASE_TOKEN` for the release pull request; no explicit
job in `release-please.yml`.** The guards workflow triggers on `pull_request`; the
release pull request is a normal pull request to `master`, and the PAT makes
GitHub run workflows on it. A dedicated job would have to check out the release
pull request's head by ref for a case the token already covers. Alternative
rejected: a `refs/pull/<n>/head` checkout job — more moving parts, same outcome.

**D5 — Archive the contended requirements in subset-before-superset order, and
hand-reconcile the one true conflict.** Two requirements are `MODIFIED` by two
changes each. `openspec archive` replaces the requirement wholesale by name, so
the last archive wins. For `imaging/filter-application` and
`document/free-transform`, the later-chronological change's delta is a strict
superset, so archiving the subset first and the superset second yields the union
with no editing. For `interop/raster-export`, the two deltas describe competing
Export-As models (format-combo vs. the CC dialog); the merged requirement is
authored from the shipped dialog in code, not from either delta. All other
capabilities are order-independent (no shared requirement names).

**D6 — Author a real Purpose for every newly created main spec.** Archiving a
change that creates a capability writes a `TBD` placeholder; `--strict` treats it
as a warning and fails. Each new capability gets a one-line Purpose derived from
its change's `proposal.md` Why. Alternative rejected: adding `## Purpose` to each
delta before archiving — same writing, more files, and no benefit once the
main spec exists.

**D7 — The gate change archives itself last.** The zero-open policy means the
change that introduces the gate must also be archived in the same pull request,
or the gate reds its own head. Its delta carries `## Purpose`, so archive copies
a real Purpose into the new `meta/openspec-archive` main spec.

## Risks / Trade-offs

- [~100 new main specs make `--strict` fail until each Purpose is written] →
  Migration step writes every Purpose before the final validation; the placeholder
  is greppable (`TBD - created by archiving change`).
- [A `MODIFIED`-vs-`MODIFIED` pair silently loses the earlier change's text] →
  subset-first order for the two superset pairs; the third is hand-merged from the
  shipped code, and all three are re-read after archiving.
- [The zero-open policy blocks a long-lived in-progress pull request] → intended;
  the change is authored, implemented, and archived within one pull request, and
  is documented in `DEVELOPING.md` and `AGENTS.md`.
- [`verify-fast.sh`'s docs-only path now runs one more script] → the script exits
  in milliseconds and reads only directory names.

## Migration Plan

1. Archive every change except the gate change and the four contended members,
   with `openspec archive <name> --yes` (proceeds through the few incomplete
   human-operation tasks).
2. Archive the superset pairs subset-first: `filter-alpha-family-contracts` then
   `port-photorust-picture-frame`; `transform-options-bar` then
   `free-transform-perimeter-rotate`.
3. Archive `export-as-dialog` and `native-webp-codec`, then hand-reconcile the
   `interop/raster-export` requirement from the shipped Export As dialog.
4. Author a Purpose for every main spec still holding the placeholder.
5. Archive the gate change, then validate.

Rollback: the whole migration is git-tracked; `git checkout openspec/` restores
the pre-migration corpus.
