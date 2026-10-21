# Design

## Approach

The gap map is already written. `docs/dev/doc-code-parity-gaps.md` holds 160
rows, each with the spec that owns the gap, a status (`MISSING`, `PARTIAL`,
`DIVERGENT`, `STALE-DOC`), an evidence `path:line`, and the GitHub issue
tracking it (#489–#651, filed under #488). The sweep's job is therefore not to
discover gaps but to make the user-facing pages point at that register, and to
make the register itself correct.

That gives the shape of the work: the roadmap becomes an index of the register
rather than a second copy of it, and the corpus stops denying the
implementation it describes.

## The roadmap

The Phase 1 / Phase 2 / Later framing is dropped. Nearly everything in Phase 1
and much of Phase 2 shipped, so a phase number no longer describes anything.

- **At a glance** — an area-to-status table. It answers "is the editor usable?"
  in one screen.
- **What works today** — feature areas, each with a path in the tree to check.
- **Open work** — thirteen sections, one per domain, roughly seventy bullets.
  A bullet is one feature cluster, not one register row, and it cites the issue
  numbers whose register rows cover it. A reviewer resolves bullet → issue →
  register row.
- **Not planned** — the deliberate absences, corrected.

The issue count is why the open list is grouped rather than exhaustive: 160
register rows written as 160 bullets would duplicate the register and read
worse. The full detail stays in the register, which the roadmap links.

### What is genuinely open

The register's rows are the source, but the sweep also re-checks the rows the
corpus and the roadmap touch. Five rows are wrong and are corrected in place:

| Row | Claim | Reality |
|---|---|---|
| `FILT-1` | Lighting Effects engine missing | Present: `crates/pictura-filters/src/render/lighting.rs`, mapped at `filter_map.rs:611`, arity at `arities.rs:80` |
| `LAY-7` | Rubylith display options missing | Present: `crates/pictura-app/cpp/mask_display_options_dialog.cpp:20`, landed in #672 |
| §E | Linked/embedded objects "verified absent" | The codec parses `lnkD`/`lnk2`/`lnk3` (`crates/pictura-codec/src/smart_object.rs:199`); the app still has no relink UI |
| §D row 2 | Artistic-filters spec says Dry Brush 1–50 | The spec now says 0–10; the spec caught up |
| §D row 3 | `export-formats.md` still says SVG is deferred | The spec now lists SVG as a first-class format |

Two more `§D` rows go with the corpus edits this change makes: the
"repository contains no source code" row is fixed here, and the
`docs/dev/STATE.md` known-risks row is partly stale.

## The corpus banner

137 files open with a blockquote whose shape is stable across the corpus:

> All module and widget names below are **design proposals**. No code exists in
> this repository. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

Only the middle sentence is false. The first sentence is still right — the
`## Rust module mapping` sections really are proposals, and #495 owns the crate
map that drifted from the workspace. The last sentence carries the source
provenance, which is the corpus's most valuable content and is untouched.

So the edit replaces one sentence, per file, with one that is true:

> The repository contains a working implementation; this file stays the
> behavioural contract for the feature, and `docs/dev/doc-code-parity-gaps.md`
> records what shipped and what did not.

The replacement is uniform on purpose: 137 files that differ only in a
mechanical way are reviewable, and a per-file status cannot be honest anyway,
because a file covers a feature area whose parts shipped at different times.

### Wrap-awareness

26 of the 137 hide the claim behind a line break, so `grep "no code exists"`
misses them:

```
> All crate, module, widget, and type names below are **design proposals**. No code
> exists in this repository.
```

Any check for this claim has to strip blockquote markers and normalise
whitespace before matching. `scripts/check-doc-claims.py` does that; a
line-oriented grep does not, which is why the check is a script and not a
one-liner in `guard.sh`.

## The guard

`scripts/guard.sh` already carries numbered rules and already calls sibling
`scripts/check-*.py` scripts for the checks that need more than a grep. Two
rules are added:

- **12** — no file under `docs/` (excluding `docs/dev/`, which is working
  notes and may quote the old claim) asserts the repository contains no code.
- **13** — every unchecked `ROADMAP.md` item carries a `#NNN` issue reference.

Rule 13 is what makes the roadmap a live index rather than a list of wishes: an
open item that names no issue is a gap nobody is tracking, and the guard says
so.

`docs/dev/` is excluded from rule 12 because the parity register quotes the
claim it is correcting, and because `docs/dev/` is working notes rather than
the behavioural contract.

## The AGENTS.md rule

The corpus drifted because nothing required it to be updated. A rule requiring
a feature change to update `ROADMAP.md` in the same pull request closes that
loop, and it is the process-side half of the same contract
`meta/documentation-corpus` states from the artefact side.

## The OpenSpec change

`meta/documentation-corpus` is a new capability under the existing `meta`
domain, which already holds the repository and development-process contracts
(`meta/commit-conventions`, `meta/openspec-layout`, `meta/agent-onboarding`).
It is a process contract about documents, so `meta` is the right home; the
change introduces no domain and no deeper path, as `openspec/config.yaml`
requires.

Scenarios are review-time, not runtime: the artefacts are prose, and the
mechanical part is enforced by `scripts/check-doc-claims.py` in the guards
workflow. The test plan therefore names `manual:` rows and says which
automated gate covers each one.
