# Tasks

## 1. Guard MSVC's per-literal limit

- [x] 1.1 Add `scripts/check-msvc-string-limits.py`: lex each `.cpp`/`.h` under
  `crates/pictura-app/cpp`, skip comments and char literals, measure every
  string-literal token (raw `R"delim(...)delim"` and ordinary `"..."`, in UTF-8
  bytes, with a lower cap for wide literals), and exit non-zero when a token body
  exceeds the limit, naming file, line, and length. Include a `--self-test` for
  the lexer's edge cases. Verify: `python3 scripts/check-msvc-string-limits.py`
  exits non-zero on the current tree and reports `theme.cpp:237`; the
  `--self-test` passes.
- [x] 1.2 Wire a new rule into `scripts/guard.sh` (mirroring the colour-literal
  rule) so the check and its self-test run from `verify-fast.sh` and the `guards`
  CI job. Verify: `bash scripts/guard.sh` prints the new check line and fails on
  the current tree.

## 2. Split the stylesheet literal

- [x] 2.1 Split the `theme.cpp` stylesheet literal into two `QStringLiteral`
  pieces joined with `+`, cutting at a CSS section boundary so the assembled text
  is unchanged. Verify: `python3 scripts/check-msvc-string-limits.py` and
  `bash scripts/guard.sh` both pass, and the QSS assembled from the two pieces is
  byte-identical to the pre-change literal extracted with
  `git show HEAD:crates/pictura-app/cpp/theme.cpp`.
- [x] 2.2 Confirm no behavior change. Verify: `tst_design_system` passes in CI
  (`bash scripts/verify-full.sh` runs it) and the byte-identity check in 2.1
  holds; `--dump-theme` (the token contract, untouched by the split) is unchanged.

## 3. Validate the fix on the Windows runner

- [x] 3.1 Temporarily relax `windows.yml`: add `crates/pictura-app/cpp/**` and
  `packaging/windows/**` to `pull_request.paths:` and drop the job `if:`, each
  with a `ponytail:` comment naming the ceiling. Verify: `git diff` shows only
  those two temporary edits.
- [x] 3.2 Push and confirm the Windows job builds this PR. Verify: the `windows`
  check is green on the PR head and its smoke check prints
  `SUMMARY passed=<n> failed=0`.
- [x] 3.3 Revert the two temporary edits once the Windows job is green. Verify:
  `git diff master -- .github/workflows/windows.yml` is empty, so the windows job
  no longer runs on an ordinary feature PR and still runs on the Release PR/tag.

## 4. Record and validate

- [x] 4.1 Update `docs/dev/STATE.md` with the fix, the guard, and the reverted
  gate. Verify: the resume anchor names the outcome and the guard; the commit
  carries `TASK-ALLOWS-DOCS`.
- [x] 4.2 Validate the change. Verify: `openspec validate --all --strict` exits 0.
