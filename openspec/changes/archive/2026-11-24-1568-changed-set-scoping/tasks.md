# Tasks

## 1. Tests

- [x] 1.1 Add `changed_files::_self_test`: merge base, working-tree diff,
      untracked files, an empty set, an unreadable list, and an affecting path.
- [x] 1.2 Extend `check-file-size.sh --self-test` with the scoped, empty-set,
      test-budget and full-scan cases.
- [x] 1.3 Extend `check-milestone-names.py --self-test` with the scoped,
      empty-set, full-scan and prefilter cases.
- [x] 1.4 Extend `check-msvc-string-limits.py --self-test` with the scoped,
      empty-set and full-scan cases.
- [x] 1.5 Add `check-colour-literals.py --self-test` with the lexer, allowlist,
      scoped, empty-set and full-scan cases.

## 2. Implementation

- [x] 2.1 Add `scripts/changed_files.py` with the shared changed set.
- [x] 2.2 Replace the per-file `wc` loop in `check-file-size.sh` with one batched
      count and add `--changed` / `--full` / `--root`.
- [x] 2.3 Add `--changed` / `--full` / `--root` to the three Python checks, each
      with a cheap file prefilter where the lexer is the cost.
- [x] 2.4 Compute the changed list once in `verify-fast.sh` via the helper and
      pass it to the file-local checks.
- [x] 2.5 Add `guard.sh --changed` and forward it to the three file-local
      checks; leave the default path whole-tree.

## 3. Gates

- [x] 3.1 Run every new `--self-test`.
- [x] 3.2 `bash scripts/guard.sh` (whole tree) exits 0 with unchanged output.
- [x] 3.3 Measure the before/after wall times on this machine.
- [x] 3.4 `openspec validate --all --strict`, `python3 scripts/check-test-plan.py`.
- [x] 3.5 Archive with `openspec archive 1568-changed-set-scoping`.
- [x] 3.6 Record the before/after measurements in `design.md`.
