# Task completion

Context: `mem:core`, `mem:suggested_commands`, `mem:conventions`.

## Done = gates pass

A coding task is done when these pass; prefer the aggregate gate over running steps by hand.

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/verify-full.sh        # CMake build + verify-fast (CI-equivalent)
# use bash scripts/verify-fast.sh when a CMake build is not needed
openspec validate --all --strict
```

For self-test work, `./build/pictura --headless --self-test` must end with `failed=0`.
Commit **only when the user explicitly asks** — never proactively.

## Every non-trivial change ships with a runnable check

Non-trivial logic (a branch, loop, parser, or money/security path) needs one runnable check: a Rust
`#[test]`, a C++ self-test check, or an `assert`-based `demo()`. Trivial one-liners need none.

## Self-test additions

- Add a new check to a `selftest_*.cpp` TU, **not** to `selftest.cpp` (the one allowlisted file —
  respect its `scripts/file-size-allowlist.txt` ceiling). Register a new TU in `cmake/sources/selftest.cmake`.
- Take the **next free code** — codes are append-only stable identifiers.
- Descriptive, milestone-free name; see `mem:app/architecture` for the token protocol.

## OpenSpec workflow

- A change lives in `openspec/changes/<kebab-name>/`: `proposal.md` (why/what/capabilities),
  `design.md` (how), `specs/<domain>/<capability>/spec.md` (ADDED/MODIFIED/REMOVED requirement deltas),
  `tasks.md` (checklist). Capabilities live in the two-level `openspec/specs/<domain>/<capability>/`
  tree (domains and the mirrored delta-path rule are in `openspec/config.yaml`); the CLI is pinned
  to 1.13.2, minimum 1.7.0 for nested specs.
- Spec format is strict: `### Requirement:` then `#### Scenario:` (exactly four `#`), normative
  SHALL/MUST wording, and at least one scenario per requirement. Validate with
  `openspec validate <change> --strict` before committing.
- Prefer a new capability name over `MODIFIED` unless the requirement itself changes.
- Archive a completed change with `openspec archive <name>` (merges its deltas into
  `openspec/specs/<domain>/<capability>/`), then `openspec validate --all --strict`.
- Run `openspec update` after a CLI upgrade to regenerate `.opencode/skills/openspec-*` and
  `.opencode/commands/opsx-*`, and commit the diff.

## Docs

`docs/` is the contract and is read-only unless the task allows it. Any docs change requires
`TASK-ALLOWS-DOCS` in the commit message (or `TASK_ALLOWS_DOCS=1`); commit docs separately. Do not
edit `docs/` to make a gate pass.
