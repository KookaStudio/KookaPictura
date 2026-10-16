# Test Plan — add-openspec-archive-gate

One row per spec scenario in `specs/meta/openspec-archive/spec.md` and
`specs/verification/verification-harness/spec.md`. Coverage names an automated
test (`<crate>::<fn>` for Rust, `tst_<suite>::<case>` for Qt Test) or
`manual: <reason>`.

| Scenario | Steps | Coverage |
|---|---|---|
| An unarchived change fails the check | Point the check at a sandbox tree holding a stray change directory | manual: `scripts/check-openspec-archived.sh --self-test` (shell gate; no crate/`tst_` test owner) |
| An empty backlog passes the check | Point the check at a sandbox tree holding only `archive/` | manual: `scripts/check-openspec-archived.sh --self-test` |
| Guards run the check | Inspect the guards-workflow step | manual: CI-only; the step is not observable off CI |
| The fast gate runs the check on every path | Inspect both branches of `scripts/verify-fast.sh` | manual: local script behavior |
| The release pull request is gated | Observe the guards workflow on the release pull request | manual: needs the live release PR and the token |
| A contributor finds the policy | Read the OpenSpec section of `DEVELOPING.md` and `AGENTS.md` | manual: prose |
| The self-test passes | Run `scripts/check-openspec-archived.sh --self-test` | manual: shell self-test |
| A regression fails the self-test | Drop the `! -name archive` clause and rerun the self-test | manual: shell self-test |
| Docs-only change | Open a docs-only pull request and observe the guards workflow | manual: CI-only |
| Code change | Open a code pull request and observe the build jobs | manual: CI-only |
| Guards fail | Make a guard exit non-zero and observe the workflow | manual: CI-only |
| The fast full gate runs the worktree-hook check | Run `scripts/verify-fast.sh` full gate | manual: local full gate / CI |
| The guards workflow runs the archive check | Observe the archive step in the guards workflow | manual: CI-only |
| Docs-only working diff | Run `verify-fast.sh` on a docs-only diff | manual: local script behavior |
| Clean tree or code change | Run `verify-fast.sh` with a code change | manual: local script behavior |
| sccache present | Run `verify-full.sh` where `sccache` is on `PATH` | manual: environment-dependent |
| sccache absent | Run `verify-full.sh` without `sccache` | manual: environment-dependent |

## Manual rows

All rows are manual: the change adds a shell gate and CI wiring, which the plan
checker cannot reference as crate/`tst_` tests. The gate's own behavior is
exercised by `scripts/check-openspec-archived.sh --self-test`; the wiring rows are
observed on the pull request's CI run.
