# Suggested commands

Run everything from the repo root. Context: `mem:core`.

## Build & run

```bash
# C++/Qt app (builds the Rust staticlib through CMake/Corrosion)
cmake -S . -B build -G Ninja && cmake --build build --parallel

# Headless self-test (offscreen QPA; implies --self-test when no document is given)
./build/pictura --headless --self-test
./build/pictura --headless --self-test <file.psd>
```

`--interop-probe` needs a real platform Vulkan instance and is **not** offscreen-compatible.

## Gates (use these instead of individual steps when you can)

```bash
bash scripts/verify-fast.sh    # fmt, clippy, test-report, file-size, guard, openspec
bash scripts/verify-full.sh    # CMake build first, then verify-fast (CI-equivalent)
bash scripts/test-report.sh [auto|always|never]   # nextest + doctests + both self-test runs
bash scripts/guard.sh          # docs guard + milestone-name + file-size checks
```

## Tests

```bash
cargo nextest run --workspace                  # preferred; -p <crate> [name] for one crate/test
cargo test --workspace --doc                   # doctests (nextest skips them)
cargo test -p pictura-render --test document_oracle          # one integration suite
cargo test -p pictura-core -- <name> --ignored --nocapture   # ignored profiling/GPU tests
```

## Lint / format

```bash
cargo fmt --all          # CI runs --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Specs / OpenSpec

```bash
openspec validate --all --strict
openspec archive <kebab-name>       # merges the change's deltas into openspec/specs/
```

## Docs guard

`docs/` is read-only unless the task allows it. A docs change requires `TASK-ALLOWS-DOCS` in the
commit message (or `TASK_ALLOWS_DOCS=1`); commit docs separately. Re-check committed docs changes
with `GUARD_BASE=<ref> bash scripts/guard.sh`.

## Misc

```bash
qmake6 -query QT_VERSION           # system Qt version
```
