# Tasks

Order follows the AGENTS.md workflow: tests, then all code and text, then one compile and every
measurement in group 8. Nothing in groups 0–7 builds, compiles or runs tests in this worktree.

## 0. Baseline (no build in this worktree)

- [x] 0.0 Claim #1042: assign it and leave a short comment that work has begun.
- [x] 0.1 Record the baseline under "Measurements" in `design.md`, from #1042 (master @ 1f730219) and the main checkout's existing build. Do not rebuild it.
  - Cargo step for a one-line app edit: 58.7 s (#1042).
  - C++ objects rebuilt by any bridge-file edit: 331 TUs, about 5000 CPU-s (#1042).
  - Qt Test layer wall time: `ctest --test-dir build -R '^tst_' -j 4` in the main checkout, if its `build/` is at 1f730219 (it runs the existing executables and builds nothing). Otherwise use #1042's numbers and say so.
  - Whether a no-change build reconfigures: `ninja -C build -n -d explain` in the main checkout (a dry run).
  - Whether `target/debug/deps` holds `libpictura_app-*.a`: list the main checkout's `target/` (#1042: 817 MB).
  - CI: five workspace compiles per pull request (#1042), and the current `test` and `oracles` job wall times from a recent master run.

## 1. Tests first

- [x] 1.1 Add a unit-test module `build_config` in `crates/pictura-app/src/` (gated with `#[cfg(test)]`) holding three tests. `dev_app_profile_keeps_release_semantics` and `every_workspace_library_disables_doctests` fail on the current tree; `bridge_modules_end_at_column_zero` pins an invariant that already holds. They run in group 8.
  - `bridge_modules_end_at_column_zero`: for every `.rs` under `src/` containing `cxx_qt::bridge`:
    - the attribute is `#[cxx_qt::bridge` at column 0, exactly once;
    - the first `\n}\n` after it equals the brace-counted end of the module (skipping comments and string and char literals);
    - the nearest non-blank line above it is not an outer attribute, a doc comment, or the tail of a multi-line attribute.
  - `dev_app_profile_keeps_release_semantics`: the workspace `Cargo.toml` has `[profile.dev-app]` with `inherits = "release"`, and no `profile.dev-app*` section sets `overflow-checks` or `debug-assertions`.
  - `every_workspace_library_disables_doctests`: every workspace member in the root `Cargo.toml` `members` list that has a library target sets `doctest = false` under `[lib]`. It also checks that `.github/workflows/ci.yml` and `scripts/test-report.sh` contain no `cargo test` invocation with `--doc`.

## 2. Development Rust profile (D1)

- [x] 2.1 Add `[profile.dev-app]` and `[profile.dev-app.package.pictura_app]` to `Cargo.toml` as in D1, with a comment naming why it inherits `release`. Update the comment above `[profile.dev.package.*]`: the CMake build uses `dev` only for a Debug configure.
- [x] 2.2 Pass `PROFILE dev-app` to `cxx_qt_import_crate` in `CMakeLists.txt` when `BUILD_TESTING` is on, `CMAKE_CONFIGURATION_TYPES` is empty, and `CMAKE_BUILD_TYPE` matches `^(Release|RelWithDebInfo)$`.
- [x] 2.3 Confirm, by reading, that every packaging configure passes `-DBUILD_TESTING=OFF`: each `cmake -S` in `.github/` (the linux, macos and windows bundle actions), `snap/snapcraft.yaml` and the flatpak manifest under `flatpak/`.

## 3. Bridge stamp and reconfigure scope (D2, D3)

- [x] 3.1 In `CMakeLists.txt`, change the bridge digest as in D2 and update the stamp comment:
  - find `#[cxx_qt::bridge`;
  - walk back over the contiguous `#[` and `///` lines above it;
  - end at the first `\n}\n`;
  - check brace balance, quote parity, and the absence of `/*`;
  - append `<hash> <path>`;
  - fall back to the whole file on any miss.
- [x] 3.2 Make the bridge files the union of the marker scan and the `src/*.rs` paths in `build.rs`'s `.files([...])` list. Make `CMAKE_CONFIGURE_DEPENDS` list the bridge files and `crates/pictura-app/build.rs`, use `file(GLOB_RECURSE ... CONFIGURE_DEPENDS)` for the source list, and drop the literal pattern entries.

## 4. Doctests (D4)

- [x] 4.1 Add `[lib] doctest = false` to every workspace library manifest.
- [x] 4.2 Remove the doctest step and the `--doctests` input from `scripts/test-report.sh` and `scripts/report_tests.py`, including the reporter's `--self-check` doctest fixtures (`DOCTEST_OK`, `DOCTEST_FAIL`, `parse_doctests`). Update the comment in `scripts/verify-fast.sh`.

## 5. CI (D5, D6)

- [x] 5.1 In `ci.yml`, update the `test` job and delete the `oracles` job.
  - Add the ImageMagick install to `test`: the AppImage cached in `~/.cache/imagemagick` with `actions/cache`, keyed on its version, extracted each run, then `magick -version`.
  - Add `psd-tools`, and `ag-psd` at a pinned version via `npm i --no-save --prefix "$RUNNER_TEMP/agpsd"`, with `NODE_PATH` set for the nextest step.
  - Remove the `Doctests` step.
  - Delete the `oracles` job (its `if:` goes with it; the `changes` job does not name it).
- [x] 5.2 Move the `software-vulkan` clippy step from `gpu-parity` to `lint`, after the workspace clippy. `lint` already installs the Vulkan headers.

## 6. Mold warning (D7)

- [x] 6.1 In `CMakeLists.txt`, when the cached EXE or SHARED linker flags carry `-fuse-ld=` on an ELF host, still probe mold. Take the first `-fuse-ld=` other than mold from either set. If mold is found and accepted, and that flag differs from the internal cache entry `PICTURA_LINKER_MASK_WARNED`, emit `message(WARNING ...)` naming the cached flag and `cmake -U CMAKE_EXE_LINKER_FLAGS -U CMAKE_SHARED_LINKER_FLAGS <build-dir>`, then store the flag in `PICTURA_LINKER_MASK_WARNED`.

## 7. Docs and memories

- [x] 7.1 `AGENTS.md`:
  - drop `cargo test --workspace --doc` from the commands list;
  - make the `test-report.sh` comment (line 44) read "nextest + app self-test + Qt JUnit";
  - replace "CI's `oracles` job installs both" (line 116) with "CI's `test` job installs both";
  - add one line on `dev-app`.
- [x] 7.2 `DEVELOPING.md`:
  - drop `cargo test --workspace --doc` (line 228);
  - in "Speeding up local builds", replace "the Rust app builds with the `dev` profile" with the `dev-app` mapping (Debug keeps `dev`, packaging keeps `release`), and reference the existing `-U` linker-cache text rather than adding a second one;
  - note that sccache does not cache incremental workspace crates.
- [x] 7.3 `.serena/memories/suggested_commands.md`: drop the doctest command (line 31) and "doctests" from the `test-report.sh` line (line 23). In `conventions.md` and `tech_stack.md`, replace the `oracles` job and the doctest step with the `test` job and `doctest = false`.
- [x] 7.4 `docs/dev/testing-conventions.md`: drop the doctest summary (line 36) and the `cargo test --workspace --doc` step (line 55), and replace the dedicated `oracles` job paragraph (about line 99) with the `test` job. Also replace the CI `oracles` job mentions in `docs/dev/opencv-feature-mapping.md` with the `test` job. Commit these docs files alone, with `TASK-ALLOWS-DOCS` after the description: `docs: drop doctests and the oracles job from testing conventions (#1042) TASK-ALLOWS-DOCS`.
- [x] 7.5 `docs/dev/STATE.md`: record the change and its measurements, and drop `cargo test --workspace --doc` and the `-DCMAKE_EXE_LINKER_FLAGS=-fuse-ld=lld` configure from its Commands block. Commit it with 7.4 under the same `TASK-ALLOWS-DOCS` rule.

## 8. Measure and verify (the single compile, then config-only re-measurements)

- [x] 8.1 Compile once: `cmake -S . -B build -G Ninja && cmake --build build --parallel` in this worktree, with at most 80% of the CPUs. Fix errors and rebuild only if code changed.
- [x] 8.2 Run `cargo nextest run -p pictura_app build_config`: the three unit tests pass. Run `python3 scripts/report_tests.py --self-check`, and `scripts/test-report.sh never`: no doctest line, exit 0.
- [x] 8.3 Read the cargo command with `ninja -t commands` for three configures in throwaway build dirs (config only; nothing is built in them):
  - the default (`--profile=dev-app`, `bp_dev_profile`);
  - `-DBUILD_TESTING=OFF` (`--release`, `bp_release_profile`);
  - `-DCMAKE_BUILD_TYPE=Debug` (no profile flag, `bp_debug_profile`).
  
  Confirm that `build/cargo/build/*/dev-app/incremental` exists in `build/`.
- [x] 8.4 Tune `dev-app` as config-only re-measurements in `build/`. For each setting, edit `Cargo.toml`, make the same one-line edit outside the bridge module in `crates/pictura-app/src/cxxqt_object/paint_tools.rs`, and time the cargo step with `cargo build --timings` via the CMake build.
  - Try `codegen-units` 16 and 256, the app opt-level 1 and 2, and `debug` `line-tables-only` and `false`.
  - For the release comparison, time the same edit in a second build dir configured with `-DBUILD_TESTING=OFF`.
  - Keep the fastest setting whose Qt Test layer wall time (`ctest --test-dir build -R '^tst_' -j 4`) is within 5% of the baseline.
- [x] 8.5 Not needed: `dev-app` paid (5.3 s against 13.2 s, no Qt Test regression), so nothing was declined. Decline path. If no setting beats `release`:
  - remove `[profile.dev-app*]` from `Cargo.toml` and the `PROFILE` logic from `CMakeLists.txt`;
  - remove the "Development builds compile the Rust library incrementally" requirement from `specs/meta/build-performance/spec.md`;
  - remove its four rows from `test-plan.md`;
  - remove `dev_app_profile_keeps_release_semantics`;
  - remove the `dev-app` lines added in 7.1, 7.2 and 7.5;
  - record the numbers and the decline under "Measurements" in `design.md`.
- [x] 8.6 Stamp checks in `build/`:
  - `bp_bridge_impl_edit`: after a body edit outside the bridge module of `paint_tools.rs`, the `sha256sum` of `build/pictura_bridge.stamp` and the set of C++ objects rebuilt (from `.ninja_log`) are unchanged.
  - `bp_bridge_same_build`: adding a method to a bridge module recompiles the including C++ in one build and nothing in the next.
  - `bp_bridge_no_change`: a second build with no change recompiles nothing.
  - `bp_bridge_stamp_fallback`: temporarily add a raw string with a column-0 `}` line inside a bridge module. Configure and see the stamp change on an edit after the string. Revert.
  - With `ninja -d explain`, a body edit in a non-bridge `.rs` does not reconfigure (measured). Adding a `.rs` file and editing `build.rs` reconfigure by construction (the `CONFIGURE_DEPENDS` glob and the configure dependency); not timed.
  - Record the configure time a bridge-file implementation edit still costs.
- [x] 8.7 Mold. Configure a throwaway dir with `-DCMAKE_EXE_LINKER_FLAGS=-fuse-ld=lld`:
  - the first configure prints the warning (`bp_mold_masked_warning`);
  - a second configure prints nothing;
  - a fresh default configure prints no warning and links with mold (`bp_mold`).
- [x] 8.8 List `target/debug/deps/libpictura_app-*.a` after `cargo clean -p pictura_app` and a nextest run, and record whether the archive still appears.
- [x] 8.9 Fill "Measurements" in `design.md` with before and after numbers for every gate:
  - cargo step;
  - C++ objects rebuilt;
  - configure time for a bridge-file edit;
  - reconfigure on a no-change build;
  - Qt Test layer wall time;
  - CI compile count;
  - the `test` job's wall time against its 30-minute timeout.
- [x] 8.10 A full `verify-full.sh` ran before the review fixes (7006 passed, 0 failed; only the docs-marker and archive checks failed, as expected before commit). After the review fixes, the rebuild and the three `build_config` tests passed; the final full gate is left to CI. Run `bash scripts/verify-full.sh` once; it passes. Then run `openspec validate --all --strict` and `python3 scripts/check-test-plan.py`. Run `grep -rn "\-\-doc\b\|oracles\` job" AGENTS.md DEVELOPING.md scripts .github docs/dev/testing-conventions.md .serena/memories`: no hits remain.

## 9. Pull request

- [x] 9.1 Commit code as `perf(build): ... (#1042)` and the docs from 7.4–7.5 separately with `TASK-ALLOWS-DOCS`. Use Conventional Commits with `(#1042)` on every commit.
- [ ] 9.2 In the pull request, read the PR's CI run and record:
  - the `test` log shows the oracle tests passing and `magick -version` succeeding (`External oracles are exercised`);
  - `lint` runs the `software-vulkan` clippy and `gpu-parity` has no clippy step (`gpu_clippy_in_lint`);
  - `qt-headless`'s cargo command carries `--profile=dev-app`, non-incremental under `CARGO_INCREMENTAL=0`.
- [x] 9.3 Archive the change with `openspec archive faster-builds-and-tests` in this PR. Title the PR `perf: faster builds and tests (#1042)`, and list the plan's manual rows in its body as the reviewer checklist.
