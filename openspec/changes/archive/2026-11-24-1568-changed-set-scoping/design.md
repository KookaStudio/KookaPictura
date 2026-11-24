# Design

## Context

See `proposal.md` — Why. The four checks are plain scripts under `scripts/`: one
bash script and three Python scripts. Each is invoked from two places today:

- `.github/workflows/guards.yml` runs `bash scripts/guard.sh` (which shells out to
  the three Python checks) and `bash scripts/check-file-size.sh` with no arguments.
- `scripts/verify-fast.sh` runs `check-file-size.sh` and `guard.sh` as two of its
  parallel independent checks (20 of them on this branch, 19 on `origin/master`).

`scripts/verify-fast.sh` already computed the changed set for its docs-only fast
path: a merge base against `master`/`main`/`kooka/*`/`origin/*`, then
`git diff --name-only` plus `git status --porcelain --untracked-files=all`. That
computation is the one to reuse, and moving it is a prerequisite for scoping
anything else.

The checks are independent by construction: each decides on a file from its own
path and text, so a changed set is a sound filter for each. `check-milestone-names`
is the only one that reads every crate source, and its whole-file scan is a pure
read; `check-msvc-string-limits` and `check-colour-literals` both live under
`crates/pictura-app/cpp`.

## Goals / Non-Goals

**Goals:**
- Make the edit-loop overhead measurable-down: the batched count and the scoped
  scans.
- One shared changed-set computation, not four.
- Scoping is never the only mode: CI and `--full` always scan the whole tree.
- An empty or unobtainable changed set falls back to the whole tree.
- `guard.sh`'s output and exit status do not change.

**Non-Goals:**
- Changing the caps, the allowlist, or what counts as a violation.
- Scoping the checks that need the whole tree by nature (the docs/roadmap
  claims, the archive check, the self-test budget): they are not file-local, and
  none of them is one of the four this issue names.
- A persistent gate manifest or a daemon (explicitly out of scope in the issue).

## Decisions

**D1 — `scripts/changed_files.py`, imported by the Python checks and run as a CLI
by the shell check.** The changed-set computation is git work, which Python does
with less quoting risk than bash; the shell check calls the CLI to a temp file
once and reads it. `git status --porcelain -z` and `git diff --name-only -z` are
used so a path with a space, a quote, or a non-ASCII byte is not quoted: a quoted
path matches no file and would silently drop it from a scoped scan (the reviewer
caught this; the self-test now pins both names and both sides of a rename).
Alternative rejected: a documented shell function copied into each check — that is
the four-copies problem the issue names. Alternative rejected: a `--changed` flag
on each check computing its own `git diff` — the repeated ~50 ms status walk and
four divergent notions of "changed".

**D2 — `--changed FILE` plus `--full`, never `--changed` alone.** `--full` is the
default and what CI uses. `--changed` takes a NUL-separated file written by the
helper so a shell caller does not have to pass a potentially huge argument list,
and so a path with a space, a quote, or a non-ASCII byte survives. Three input
cases are distinguished: a missing file is a usage error (a typo must not silently
widen the scan), an empty file is the empty-set fallback, and a non-empty file
with no NUL record is a hand-written newline list `check-file-size.sh` cannot
split safely, so it is also a usage error rather than a silent pass. The Python
checks still accept the newline form, because they can tell where a name ends.

**D3 — The empty set scans everything.** `scoped_paths` returns `None` for "no
list given" and "empty list", and every caller treats `None` as whole tree. That
is the invariant that stops a check from going quiet when a detached HEAD, a
shallow clone, or a clean tree makes the set unobtainable. It is also why the
self-test's empty-list case asserts a failure.

**D4 — A check's own allowlist change widens the scan.** `check-file-size.sh`
reads `scripts/file-size-allowlist.txt`; `check-colour-literals.py` reads
`scripts/colour-literal-allowlist.txt`. Changing an allowlist can affect any file
the changed set does not name (a newly exempted or newly re-capped file), so both
fall back to a whole-tree scan when their allowlist is in the set. The
alternative — trusting the set — would let an allowlist edit pass with a stale
violation. `check-milestone-names.py` and `check-msvc-string-limits.py` have no
allowlist and need no such rule.

**D5 — The batched count keeps the cap/allowlist logic in one awk pass.**
`wc -l` over the NUL-separated path list emits `lines path` per file plus a
`total` line; one `awk` splits that, applies the path-based budget
(`*/tests/*`, `*/tests.rs`, `*_test.{cpp,h,rs}` → 1400, else 1200), and compares
against the allowlist. This replaces the per-file bash loop *and* the per-file
`wc` fork in one move, so the 2,506-fork cost disappears whether or not the run
is scoped. `awk` is already a dependency of the repo's scripts; no new one.

**D6 — `guard.sh` gains `--changed`, CI keeps calling it bare.** The issue asks
that guard's output and exit status not change. Guard aggregates the three Python
checks plus git-grep-based checks that are inherently whole-tree; forwarding the
list only to the three file-local checks keeps the aggregate's semantics (it
still fails on any violation it would have found, because the checks fall back to
`--full` themselves when they cannot scope). CI's `bash scripts/guard.sh` is
unchanged, so the guards workflow is whole-tree. `verify-fast.sh` passes the
list, so the local loop is scoped.

**D7 — `check-test-plan.py` is left alone.** It already scopes to the changes
that touch `openspec/changes/`, using its own merge-base logic. Rewiring it onto
the helper would change `GUARD_BASE` semantics it shares with CI, for no
measurable win (the walk is one directory). Out of scope for this issue.

**D8 — `check-milestone-names.py` gets a cheap prefilter, not just scoping.** Its
lexer costs ~3.3 s over 29.8 MB even after the list is read. A whole-file
`[mM]<digit>` regex is a strict superset of every milestone match (all three
patterns contain `m`/`M` and a digit), so a file without one cannot hold a
violation; 115 of 2,506 files pass it. The full-tree scan drops to ~0.52 s with
no change in findings. Verified by comparing every file's violations against the
old script, not by reasoning.

**D9 — `check-msvc-string-limits.py` gets a cheap prefilter too.** A literal can
only exceed the limit if a physical line is long, the file has a raw literal
(`R"`, whose body may hold newlines), or it has a backslash-newline continuation.
That skips 1,059 of 1,091 files. The bound counts *characters* while the limit
counts *bytes*, so it assumes four bytes per source character (`LIMIT // 16` =
1,000): a wide `L` literal holding 2,001 emoji is 2,024 characters but 8,004
bytes, past MSVC's 8,000-byte `L` cap, and a 4,000-character bound would miss it.
The reviewer caught exactly that case; the self-test now pins it. Verified
file-by-file against the old lexer at three thresholds (16,000, 200, 40) so the
comparison had actual hits to compare.

## Risks / Trade-offs

- [A scoped run misses a violation the whole-tree run would catch, and someone
  relies on the local gate alone] → the scoped mode is a local-only optimisation;
  CI and `guard.sh` without a flag are whole-tree, `--full` is the default, and
  each self-test asserts the `--full` failure alongside the scoped pass.
- [The changed set from a merge base can be large on a long-lived branch] → it
  only narrows; a large set is a large scan, which is what happens today anyway.
- [A path containing a newline cannot round-trip through a line-delimited list]
  → the helper writes and every shell reader (`verify-fast.sh`'s docs-only scan
  and `check-file-size.sh`) reads NUL records, so a name holding a newline, a
  space, or a non-UTF-8 byte round-trips. git output is decoded with
  `surrogateescape`, so a non-UTF-8 name cannot abort the gate with a
  `UnicodeDecodeError`; the self-test asserts a space, a non-ASCII name, a
  non-UTF-8 name, and both sides of a rename.
- [The MSVC prefilter could miss a literal split across lines with an escape] →
  the backslash-newline test covers the continuation case; a literal longer than
  16,000 bytes cannot be assembled from short physical lines without one.
- [`guard.sh --changed` changes what a local run reports] → it reports a strict
  subset locally and the full set in CI; the flag is opt-in and documented in the
  script header.

## Migration Plan

No migration. The default of every check is unchanged; `--changed` is additive.
Rollback is reverting the six script edits.

## Measurements

All figures are from this machine (16 CPUs, `git` 2.55, bash 5.3, Python 3.13),
on an otherwise idle box. Wall times are `date +%s%N` deltas around one script
run, taken after a warm-up run. "N runs" says how many; the value is the median
where more than one is listed, and the spread is the min-max. Load was not
introduced deliberately: the box was idle, so these are the *idle* numbers the
issue asks for; a loaded box is noisier in the same direction for both sides.

### Before (origin/master 515ab3a0)

| what | value |
|---|---|
| `check-file-size.sh` wall time, full | 4,188 / 4,303 / 5,902 ms (3 runs, median 4,303) |
| `check-file-size.sh` `wc` processes | 2,506 (one per file) |
| `check-milestone-names.py` wall time, full | 3,510 / 3,503 ms (2 runs, median 3,507) |
| `check-msvc-string-limits.py` wall time, full | 3,592 / 3,640 ms (2 runs, median 3,616) |
| `check-colour-literals.py` wall time, full | 65-69 ms (5 runs, median 67) |
| one-file edit, four checks, whole tree | ≈ 11.5 s |

### After (branch build/1568-changed-set-scoping)

| what | value | delta |
|---|---|---|
| `check-file-size.sh` wall time, full | 34-35 ms (5 runs, median 35) | **×123 faster** |
| `check-file-size.sh` `wc` processes, full | 1 | 2,506 → 1 |
| `check-file-size.sh` wall time, scoped | 13-15 ms (3 runs, median 14) | ×307 vs the old full run |
| `check-file-size.sh` `wc` processes, scoped | 1 (+1 `tr`/`wc` guard on the list) | |
| `check-milestone-names.py` wall time, full | 548-555 ms (3 runs, median 554) | **×6.3 faster** |
| `check-milestone-names.py` wall time, scoped | 59-61 ms (3 runs, median 60) | ×58 vs the old full run |
| `check-msvc-string-limits.py` wall time, full | 293-305 ms (3 runs, median 296) | **×12 faster** |
| `check-msvc-string-limits.py` wall time, scoped | 53-55 ms (3 runs, median 54) | ×67 vs the old full run |
| `check-colour-literals.py` wall time, full | 110-126 ms (3 runs, median 115) | slightly slower (see note) |
| `check-colour-literals.py` wall time, scoped | 53-58 ms (3 runs, median 55) | ≈ same |
| one-file edit, four checks, scoped | ≈ 183 ms (14 + 60 + 54 + 55) | **≈ ×63 faster** than 11.5 s |

The `wc` process counts are measured with a `wc` shim on `PATH` that appends a
line per invocation, not with `strace` (it is not installed here).

`check-colour-literals.py` full-tree is about 40 ms slower (67 → 115 ms) because
it now lists the candidate paths through `Path.rglob` *and* filters them by the
relative path, where the old loop checked the name it already had. It is still
under 0.13 s and the scoped path is unchanged, so the regression is recorded
rather than chased; if it ever matters, the `wanted()` check can move into the
`rglob` loop.

The batched `wc -l` costs about 13 ms more than the bare `find | xargs wc -l`
(roughly 35 ms vs 20 ms) because the full count is written to a temp file and read
back so its record count can be compared with the input. That check is what turns
"wc could not count one file" into a failure instead of a silently dropped file;
13 ms for a check that cannot go quiet is the right trade.

`check-file-size.sh --self-test` also covers a non-empty newline-only list (a usage
error, not a silent pass) and a name holding a newline (wc quotes it, the record
count still matches, and the violation is still reported).

`guard.sh`'s whole-tree output is byte-identical to `origin/master`'s on the same
tree, and its exit status is the same (0): verified by running the
`origin/master` copy of `guard.sh` and of the three checks it shells out to, then
diffing the two runs' stdout+stderr. Each check's own whole-tree output is also
byte-identical to the old script's on the same tree (diffed for all four).

One whole-gate caveat: `bash scripts/verify-fast.sh` fails on this branch because
`crates/pictura-app/cpp/frame.h` is 804 lines against `check-shell-seams.py`'s
760-line cap, which is already true at `origin/master` 515ab3a0 and untouched by
this change. Every check this change touches passes, and the whole gate's other
18 checks are green.
