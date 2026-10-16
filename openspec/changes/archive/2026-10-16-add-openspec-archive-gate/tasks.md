# Tasks

## 1. Archive-gate check

- [x] 1.1 Add `scripts/check-openspec-archived.sh`: with no argument, scan `openspec/changes`; with a directory argument, scan that directory; fail (print each offending path) when a directory other than `archive` exists directly under it, exit 0 otherwise — verify by pointing it at a `mktemp -d` tree containing a stray change (non-zero) and one containing only `archive` (zero)
- [x] 1.2 Add `--self-test` to the script: build a `mktemp -d` sandbox, assert the fail case and the pass case, `trap` cleanup, exit non-zero on a failed assertion — verify `bash scripts/check-openspec-archived.sh --self-test` exits 0
- [x] 1.3 Deliberate-failure check: drop the `! -name archive` clause, confirm `--self-test` fails and names the assertion, restore — verify the self-test is non-vacuous (broken non-zero, restored zero)

## 2. CI and local wiring

- [x] 2.1 Add a guards-workflow step running `scripts/check-openspec-archived.sh --self-test` then `scripts/check-openspec-archived.sh` after the OpenSpec-validation step — verify the YAML parses and the commands match the neighboring script steps
- [x] 2.2 Add `bash scripts/check-openspec-archived.sh` to the `scripts/verify-fast.sh` docs-only path — verify the added line matches the guard/validation block style
- [x] 2.3 Add `bash scripts/check-openspec-archived.sh --self-test` and `bash scripts/check-openspec-archived.sh` to the `scripts/verify-fast.sh` full gate next to `check-selftest-budget.sh` — verify by reading the block and running `bash -n scripts/verify-fast.sh`

## 3. Policy documentation

- [x] 3.1 Add the zero-open policy to the OpenSpec section of `DEVELOPING.md` (a change is authored, implemented, and archived in the pull request that ships it; non-archived changes fail `scripts/check-openspec-archived.sh`) and add the script to the list of gate commands — verify the added commands run as written
- [x] 3.2 Add the same policy to the Spec-workflow section of `AGENTS.md` — verify the wording matches `DEVELOPING.md`
- [x] 3.3 Confirm no `TASK-ALLOWS-DOCS` marker is needed (root `DEVELOPING.md`/`AGENTS.md`, not `docs/`) — verify `bash scripts/guard.sh` passes without the marker

## 4. Backlog migration

- [x] 4.1 Archive every change except the gate change and the four contended members with `openspec archive <name> --yes` — verify each invocation exits 0 and the directories move under `archive/`
- [x] 4.2 Archive the superset pairs subset-first (`filter-alpha-family-contracts` then `port-photorust-picture-frame`; `transform-options-bar` then `free-transform-perimeter-rotate`) — verify the surviving `imaging/filter-application` and `document/free-transform` requirements equal each superset delta
- [x] 4.3 Archive `export-as-dialog` and `native-webp-codec`, then hand-reconcile the `interop/raster-export` "Export As writes the flattened composite" requirement to the shipped Export As dialog (dialog targets + WebP through the engine + WebP alpha) — verify the requirement and its scenarios match the code's Export As behavior
- [x] 4.4 Author a one-line `## Purpose` for every main spec still holding `TBD - created by archiving change`, deriving each from its change's `proposal.md` Why — verify `grep -rl 'TBD - created by archiving change' openspec/specs` returns nothing

## 5. Integration and self-archive

- [x] 5.1 Run `openspec validate --all --strict` — verify it exits 0 with no warnings
- [x] 5.2 Self-archive the gate change with `openspec archive add-openspec-archive-gate --yes` — verify the change is under `archive/` and `openspec/specs/meta/openspec-archive/spec.md` carries a real Purpose (not the placeholder)
- [x] 5.3 Verify `bash scripts/check-openspec-archived.sh` and `bash scripts/check-openspec-archived.sh --self-test` both exit 0 on the migrated tree
- [ ] 5.4 Run `bash scripts/verify-fast.sh` (CI also runs it and the guards workflow) — verify it is green; note in the task result that the diff is not docs-only, so the full gate runs
