# Conventions

Project-wide invariants: `mem:core`. Test mechanics in full: `docs/dev/testing-conventions.md`.
Commands: `mem:suggested_commands`.

## Editing rules

- **No comments explaining what the code says.** Comment only non-obvious intent. Mark deliberate
  shortcuts with a `ponytail:` comment naming the ceiling (e.g.
  `// ponytail: global lock, per-account locks if throughput matters`).
- **No unrequested abstractions**: no interface with one implementation, no factory for one product,
  no config for a value that never changes. Delete over add.
- **No new dependencies** without stating why in the commit/task result. Prefer std + present crates.
- **Behavioral parity only where an oracle exists**; mark approximations and never claim verified
  parity without a proving test. **Fix seeds** for anything random; GPU is an accelerator, not an
  oracle — golden comparisons run on CPU.
- **Escalate, don't guess**: if a spec is ambiguous or a dependency/toolchain decision is needed,
  stop and report the blocker rather than inventing scope.

## File size

- Target **< 800 LOC**; hard cap **1200** for code, **1400** for tests.
- Tests are detected by path: Rust under `tests/` or named `tests.rs`; C++ `*_test.{cpp,h}`.
- A file over its cap must be listed in `scripts/file-size-allowlist.txt` (a ceiling that only
  shrinks) and checked by `scripts/check-file-size.sh` (inside `verify-fast.sh`).
- Split along class/concern seams with **pure moves** (no behavior change): C++ classes may span
  several `.cpp` TUs; Rust modules become submodule directories.

## Testing (no framework anywhere)

- Rust: std `#[test]` only. **Unit tests** are inline `#[cfg(test)] mod tests` in the same file;
  **integration/oracle tests** are `crates/*/tests/*.rs`. Test names are snake_case phrases naming
  the guarantee (`length_mismatch_is_an_error`).
- **`#[ignore = "..."]`** is only for manual profiling / GPU-mandatory tests, and the reason string
  names how to run it (`--ignored --nocapture`). Never use `#[ignore]` to hide unimplemented work.
- **Oracle tests are differential** (our impl vs a second independent impl). Rules:
  - **Self-skip, never fail-to-run**: if the external tool is missing, `eprintln!("skipping: ...")`
    and `return`. CI's `test` job installs `magick`, `psd-tools`, and `ag-psd` so they run for real.
  - **Prove the harness has teeth**: include a test that a wrong output actually fails.
  - Compare through `pictura-testkit::compare` with an explicit per-class tolerance; never hand-roll
    a byte diff.
  - The Rust test owns the comparison; the Python script owns the ImageMagick/psd-tools call
    (`Command::new("python3").arg(script())`).
  - **Tolerance honesty**: where an external tool's algorithm genuinely differs, exclude the mode
    from the oracle and cover it with hand-computed unit tests; record the reason in the crate's
    `tests/README.md`.
- **Fixtures** under `crates/*/tests/fixtures/` are generated, deterministic, and committed;
  regeneration must not churn the tree.
- **Definition of done**: non-trivial logic (a branch, loop, parser, or money/security path) ships
  with one runnable check — a unit test, a `demo()` self-check, or an integration test. Trivial
  one-liners need none.

## Sub-topic

- The app's C++ self-test token protocol, exit-code-as-identity, append-only codes, and the
  `selftest.cpp` allowlist: `mem:app/architecture`.
