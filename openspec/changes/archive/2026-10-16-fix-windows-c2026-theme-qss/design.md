# Design

## Context

See `proposal.md` — Why. Current state that shapes the approach:

- `theme.cpp:237-494` builds the stylesheet from one `QStringLiteral(R"(...)")`
  literal, now ~19,670 bytes. MSVC's limit is 16,380 single-byte characters per
  literal, applied *before* adjacent literals are concatenated, so splitting the
  literal into pieces is a valid fix (the standard workaround).
- `windows.yml:41` runs the Windows job only on the release-please PR, so a
  Windows-only compile error cannot surface on a feature PR (#461 made the same
  observation for `macos.yml` and relaxed-then-restored its gate).
- `scripts/guard.sh` already hosts C++ source checks and is run by both
  `verify-fast.sh` and the `guards` CI workflow, so a new check wired there runs
  everywhere without touching CI wiring.

## Goals / Non-Goals

**Goals:**

- `theme.cpp` compiles under MSVC with the stylesheet text unchanged.
- A guard catches the same class of breakage on Linux, before a Windows runner
  is needed, and does not falsely pass a single literal over the limit.

**Non-Goals:**

- Making the Windows build run on C++ path PRs permanently (deliberately
  temporary here; the equivalent `appimage.yml` gate is a separate follow-up).
- Rewriting the stylesheet generator or changing any QSS token (no behavior
  change).

## Decisions

### Split the stylesheet with separate `QStringLiteral` calls

Replace the one literal with two, joined by `+`:

```cpp
QString qss = QStringLiteral(R"(
... first half ...
)") + QStringLiteral(R"(
... second half ...
)");
```

- **Why two separate tokens:** MSVC measures each literal *before* adjacent
  concatenation, so both `QStringLiteral(...) + QStringLiteral(...)` and a single
  `QStringLiteral(R"(A)" R"(B)")` clear the cap. The `+` form keeps each piece an
  independent literal token, matching the issue's direction and making the guard's
  per-token measurement exact. The runtime `+` is one string concatenation per
  stylesheet build (on theme apply), which is negligible.
- **Two pieces:** splitting the ~19,670 bytes in half minimises the largest
  piece (~9,835 bytes, ~6 KB of headroom). The seam sits at a CSS section
  comment so each piece stays readable.
- **Byte-identity:** the two raw-string bodies concatenate to exactly the
  original body; nothing is added at the seam.

### Guard MSVC's per-literal limit in ordinary CI

Add `scripts/check-msvc-string-limits.py` and a rule in `scripts/guard.sh`
(mirroring the colour-literal rule). The script lexes each `.cpp`/`.h` under
`crates/pictura-app/cpp`, skipping comments and char literals, and measures each
string-literal token's body: raw `R"delim(...)delim"` and ordinary `"..."`. It
ships a `--self-test` for the tricky lexer cases (comments, char literals, C++14
digit separators, raw delimiters, encoding prefixes, UTF-8), and the guard rule
runs it before the scan.

- **Why a script and not a compiler flag:** GCC/Clang's `-Woverlength-strings`
  warns against the standard's minimum (4,095 characters), far below MSVC's
  16,380, so it cannot express this limit.
- **Why measure per token:** that is the unit MSVC measures (pre-concatenation),
  so the guard agrees with the compiler — a token under the limit passes even if
  the concatenation is larger, exactly as MSVC allows.
- **Threshold 16,000:** MSVC's cap is 16,380 bytes applied *before* adjacent
  literals are concatenated (the post-concatenation maximum that VS2022 17.0
  lifted does not apply to a single literal). A round 16,000 leaves a small
  margin and stays unambiguous. A literal over it fails.
- **Reporting unit:** UTF-8 bytes of the source body. For raw strings this equals
  the runtime bytes; for ordinary literals escapes only shrink, so source bytes
  are a conservative upper bound. UTF-8 source (e.g. the `▾` glyph in
  `theme.cpp`) is counted as bytes, not code points. Wide literals (`L`/`u`/`U`)
  are held to `16,000 / width` because MSVC's cap counts single-byte characters.

### Temporary PR gate, reverted after the fix

Add `crates/pictura-app/cpp/**` and `packaging/windows/**` to `windows.yml`'s
`pull_request.paths:` and drop the release-please-only job `if:`, each carrying a
`ponytail:` comment naming the ceiling. Both edits are reverted within this
change once the Windows job is green on the PR; `.github/workflows/windows.yml`
has no net diff against `master`.

- **Why:** the Windows/MSVC compile is the only oracle for `C2026`; Clang/GCC
  accept the long literal, so the fix cannot be validated by the Linux gate. The
  runner is expensive, so the relaxation is scaffolding, not the permanent
  policy.
- **Consequence:** after the revert the PR's Windows job is skipped again, so the
  green run lives on an intermediate commit — the same acceptance #461 recorded.

## Risks / Trade-offs

- A guard false negative for a literal assembled by a macro → the guard measures
  source tokens; macros that build a literal are not literal tokens, but no shell
  source does this. Ceiling named; split the literal if one appears.
- A guard false positive for an ordinary literal with escapes near the threshold
  → source bytes over-count the decoded size; split or rename the literal. Not
  expected in this codebase.
- A seam typo changes the stylesheet → mitigate by comparing the assembled QSS
  against the pre-change literal and by `tst_design_system` / `--dump-theme`.
- MSVC cap ambiguity (16,380 vs 16,384) → the 16,000 threshold is below both.

## Migration Plan

1. Split the `theme.cpp` literal; confirm the guard passes and the assembled QSS
   is byte-identical.
2. Add `scripts/check-msvc-string-limits.py`; wire the `guard.sh` rule; confirm
   the guard fails on the pre-split literal.
3. Apply the temporary `windows.yml` gate/paths; push; let the Windows job build
   this PR.
4. Once the Windows job is green, revert the gate/paths; confirm
   `.github/workflows/windows.yml` matches `master`; record the revert.
5. Update `docs/dev/STATE.md`.

## Open Questions

None.
