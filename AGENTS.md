# Agent guide — Kooka Pictura

Documentation-first reimplementation of **Adobe Photoshop CS6** (v13) in
**Rust + Qt6** on Linux. `docs/` is the long-form behavioral contract; code
lives in `crates/`.

## Start here

- `docs/dev/STATE.md` — resume anchor ("where things are"). Read it first;
  update it at the end of a milestone.
- `docs/dev/testing-conventions.md` — how every test layer is built and reports.
  The source of truth for test mechanics; don't restate it, follow it.
- `DEVELOPING.md` — onboarding: build, architecture, common tasks, OpenSpec CLI.
- `CONTRIBUTING.md` — provenance, asset, and dependency rules (not optional).

## Layout

```
docs/          spec corpus + docs/dev/ working notes (read-only unless tasked)
openspec/      change proposals (changes/) + capability specs (specs/)
crates/        10 engine crates + the app
  core|codec|color|adjust|filters|ops|paint|select   spec math, no Qt
  pictura-render/   CPU compositing + GPU (wgpu/Vulkan) compute
  pictura-testkit/  golden-image compare / hashing
  pictura-app/      the only Qt crate — Rust bridge (src/cxxqt_object/*) + C++/Qt (cpp/)
scripts/       verification gates + Python oracle tools
```

The app is a Rust `staticlib` (`pictura_app`, built by Corrosion from Cargo)
linked into the C++ `pictura` binary. cxx-qt codegen is driven by
`crates/pictura-app/build.rs` + `src/cxxqt_object.rs`. **New `.cpp`/`.h` files
must be added to `CMakeLists.txt` explicitly — there is no globbing.**

## Commands

```bash
cargo fmt --all                                  # CI runs --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace                    # one crate/test: -p <crate> [name]
cargo test --workspace --doc                     # doctests; nextest skips them
cargo test -p pictura-render --test document_oracle       # one integration suite
cargo test -p pictura-core -- <name> --ignored --nocapture # ignored profiling/GPU
bash scripts/test-report.sh                      # nextest + doctests + app self-test + Qt JUnit -> unified report
bash scripts/verify-fast.sh                      # fmt, clippy, test-report, file-size, guard, openspec
bash scripts/verify-full.sh                      # CMake build first, then verify-fast
openspec validate --all --strict                 # openspec 1.13.2
cmake -S . -B build -G Ninja -DCMAKE_EXE_LINKER_FLAGS=-fuse-ld=lld && cmake --build build --parallel
./build/pictura --headless --self-test [file.psd]
```

Toolchain is pinned by `rust-toolchain.toml` (1.98). Qt is the system Qt 6
(`qmake6 -query QT_VERSION`). Cargo links with `lld` (`.cargo/config.toml`);
CMake builds with Ninja. `deny.toml` gates licenses and dependencies; CI runs
`cargo deny check`.

## Serena (symbol intelligence)

The repo is already a Serena project: commit `.serena/project.yml` and
`.serena/memories/*`; only `cache/`, `project.local.yml`, and
`compile_commands.json` are gitignored. Prefer its LSP tools (`find_symbol`,
`find_referencing_symbols`, `replace_symbol_body`, `rename_symbol`) over text
search, and read the relevant memory before a non-trivial task.

- **Index upkeep.** `serena project index` is needed only once, after a fresh
  clone; afterwards Serena updates the index itself as files change.
- **C++ needs a compile database.** clangd reads `compile_commands.json` from the
  repo root, linked there by the CMake configure; if C++ navigation looks fuzzy,
  (re)configure first.
- **Keep memories current.** Update `.serena/memories/*` (via `write_memory`) when
  conventions or architecture change.

## Headless mode

`./build/pictura --headless` selects the offscreen QPA plugin before
`QApplication` and implies `--self-test` when no document is given, so it never
blocks. `--interop-probe` requires a real platform Vulkan instance and is **not**
offscreen-compatible. See `DEVELOPING.md` for the self-test token protocol.

## Verification (the non-obvious parts)

- **The C++ self-test is a hand-rolled oracle**, not Qt Test:
  `crates/pictura-app/cpp/selftest*.cpp` + `selftest_report.*`, one long
  sequential `runSelfTest()`. It emits one token per check to stderr
  (`pictura self-test: PASS|SKIP|FAIL <suite> <name>`), closed by
  `SUMMARY passed=<n> failed=<n> skipped=<n>`. **The exit code is the failure
  identity** — `ST_FAIL(code)` returns its code; codes are append-only, and
  names carry no milestone.
- Add a check with `ST_BEGIN` / `ST_PASS` / `ST_SKIP` / `ST_FAIL` / `ST_FINISH`,
  take the next free code, and keep each `selftest*.cpp` inside its
  `scripts/file-size-allowlist.txt` ceiling.
- **Qt Test is the GUI growth path.** The app C++ builds as a `pictura_shell`
  static library (every app source except `main.cpp` + `selftest*`); suites in
  `crates/pictura-app/cpp/tests/tst_*.cpp` link it plus `Qt6::Test`, gated on
  `BUILD_TESTING`, and run offscreen via
  `ctest --test-dir build -R '^tst_' --output-on-failure`.
  `scripts/test-report.sh` runs CTest and folds
  `build/qt-test-results/*.xml` into the unified report. New GUI checks go here
  — the self-test only shrinks (retired exit codes are never reused). The
  mechanical guard is `scripts/check-selftest-budget.sh` with the lower-only
  budget in `scripts/selftest-budget.txt`, run by `verify-fast.sh` and the
  guards CI workflow.
- Rust tests are std `#[test]` only (no framework). Oracle tests self-skip when
  `magick` / `psd-tools` are absent; CI's `oracles` job installs both. Profiling
  and GPU tests are `#[ignore]`d with a reason string (`--ignored --nocapture`).

## Spec workflow (OpenSpec)

[OpenSpec](https://github.com/Fission-AI/OpenSpec) turns work into reviewable
change proposals before code. `docs/` remains the long-form contract; OpenSpec
carries the per-change *requirements* and task list. **It is mandatory for
non-trivial changes and works from the CLI, with or without an agent** (the
`/opsx-*` commands wrap the same CLI; see `DEVELOPING.md`).

- A change lives flat in `openspec/changes/<kebab-name>/`:
  `proposal.md` (why / what / capabilities), `design.md` (how),
  `specs/<domain>/<capability>/spec.md` (ADDED / MODIFIED / REMOVED deltas),
  `tasks.md` (checklist). Only the delta path inside a change nests.
- Capabilities live in `openspec/specs/<domain>/<capability>/spec.md`; a
  capability's id is its path relative to `specs/` (e.g.
  `compositing/layer-compositing`). Taxonomy and the mirrored delta-path rule
  are in `openspec/config.yaml`. Prefer new capability names over `MODIFIED`
  unless the requirement itself changes.
- Format is strict: `### Requirement:` then `#### Scenario:` (exactly four `#`),
  normative SHALL/MUST wording, at least one scenario per requirement. Validate
  with `openspec validate --all --strict` before committing.
- CLI pinned to 1.13.2 (minimum 1.7.0 for nested specs); CI installs the same
  pin. Run `openspec update` to regenerate `.opencode/skills/openspec-*` and
  `.opencode/commands/opsx-*` after an upgrade, and commit the diff.
- Archive a completed change with `openspec archive <name>` to merge its deltas
  into `openspec/specs/`. Pre-OpenSpec milestone work (m0…m47) is retroactively
  archived under `openspec/changes/archive/`.

## Git conventions

- **Commits** — [Conventional Commits](https://www.conventionalcommits.org/):
  `type(scope): description (#issue)`. Valid types: `feat`, `fix`, `docs`,
  `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`
  (no `bug` — use `fix`). Scope is optional but preferred (the crate, module,
  or system affected).
- **Every commit ends with its GH issue number in parentheses**, e.g.
  `fix(codec): RLE row padding (#12)`. Open or reuse an issue first. A `docs/`
  change also carries `TASK-ALLOWS-DOCS` (guard rule 4) after the description:
  `docs: record free-transform-quad (#21) TASK-ALLOWS-DOCS`.
- **Claim the issue when you start.** When you open a branch for an issue,
  assign it to yourself and leave a short comment saying work has begun, so
  nobody duplicates the effort.
- **Branch names** — `type/issue-number-kebab-description`, e.g.
  `feat/14-port-release-please`; include the issue number after the slash.
- **GH issue titles** — the conventional prefix, no number, e.g.
  `feat: port the healing family from photorust`.
- **PR titles** — conventional prefix + issue number in parentheses, e.g.
  `feat: port the healing family (#10)`.
- **Enforcement** — CI checks PR titles and every PR commit (`pr-conventions`
  workflow, Conventional Commits with the type list above); the title check
  fails on a malformed title. Squash-only merging makes the PR title the commit
  release-please reads. It blocks merge only once `Conventional PR title` is a
  required status check on `master`. The `(#N)` suffix and `TASK-ALLOWS-DOCS`
  stay review-enforced; local `commit-msg` hooks are still tracked in issue #69.

## Rules

1. **Specs are the contract.** Do not change `docs/` unless the task says so;
   any `docs/` change needs `TASK-ALLOWS-DOCS` or `scripts/guard.sh` fails.
   Commit docs separately; `GUARD_BASE=<ref>` re-checks committed docs changes.
2. **Definition of done.** Non-trivial logic (a branch, loop, parser, or
   money/security path) ships with one runnable check: a unit test, a `demo()`
   self-check, or an integration test. Trivial one-liners need none.
3. **No unrequested abstractions.** No factory for one product, no trait with one
   implementation, no config for a constant. Delete over add.
4. **No new dependencies** without stating why in the commit/task result. Prefer
   std and already-present crates.
5. **Behavioral parity only where the oracle exists.** Adobe's closed algorithms
   are approximated and marked as such; never claim verified parity without a
   test that proves it.
6. **Determinism.** Golden comparisons run on CPU; GPU is an accelerator, not an
   oracle. Fix seeds for anything random.
7. **No comments explaining what the code says.** Comment only non-obvious
   intent. Mark deliberate shortcuts with a `ponytail:` comment naming the
   ceiling.
8. **Escalate, don't guess.** If a spec is ambiguous or a dependency/toolchain
   decision is needed, stop and report the blocker rather than inventing scope.
9. **File size.** Target under **800 LOC**; hard cap **1200** for code and
   **1400** for tests. Tests are detected by path: Rust under `tests/` or named
   `tests.rs`, C++ `*_test.{cpp,h}`. A file over its cap must be listed in
   `scripts/file-size-allowlist.txt`, a ceiling that only shrinks. Split along
   class/concern seams with pure moves (no behavior change).
10. **Milestones (`mNN`) live only in comments, docs, and specs** — never in
    identifiers, string literals, or test names. `scripts/check-milestone-names.py`
    (run by `guard.sh`) enforces this.
11. **GUI test migration guard.** New GUI checks are Qt Test cases under
    `crates/pictura-app/cpp/tests/`, never new `runSelfTest()` checks; the
    self-test only shrinks and retired exit codes are never reused (see
    Verification).

## Anti-patterns

- Weakening or deleting a test to make CI pass. Changing a golden baseline
  requires an explicit note in the task result.
- Committing build artifacts (`target/`, `build/`, `.opencode/node_modules/`).
- Touching unrelated files. Keep diffs scoped to the task.
