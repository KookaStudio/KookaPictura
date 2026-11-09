# Tasks

## 1. Extract the shell source list

- [x] 1.1 Create `cmake/sources/` and the 14 shell fragments from the current `PICTURA_SHELL_SOURCES`; remove the two pre-existing duplicate lines (`panels/character_panel.cpp`, `panels/paragraph_panel.cpp`); verify the concatenated sorted paths equal the old 556 unique shell paths, and do not add the 40 unlisted headers or the Rust-built `panels/actions_tool_recording_bridge.cpp`
- [x] 1.2 Replace the `set(PICTURA_SHELL_SOURCES ...)` block with `include()` calls placed before `add_library`; verify configure succeeds and `add_library` plus `pictura_depend_on_bridge_stamp` still receive the variable
- [x] 1.3 Verify the root `CMakeLists.txt` still has both targets, the `filter_commands.cpp` per-source property, no `file(GLOB` over C++ sources, and the `x-release-please-version` marker

## 2. Extract the self-test source list

- [x] 2.1 Create `cmake/sources/selftest.cmake` with `main.cpp` and `selftest*`; verify it holds 76 entries and no self-test source appears in a shell fragment
- [x] 2.2 Replace the `set(PICTURA_SELF_TEST_SOURCES ...)` block with the include before `add_executable`; verify configure succeeds and the bridge-stamp call receives the list
- [x] 2.3 Add `scripts/check-cmake-source-fragments.py` with a `--self-test`: fail on a duplicate path, a listed path missing from disk, a non-test non-Rust-built `.cpp` not listed, and a `main.cpp`/`selftest*` source in a shell fragment
- [x] 2.4 Wire the check into `scripts/verify-fast.sh` and `.github/workflows/guards.yml`; verify it passes on the unchanged tree and fails on a planted duplicate and a planted missing path

## 3. Update contributor documentation

- [x] 3.1 Update `AGENTS.md` (:33) and `DEVELOPING.md` (:102, :156, :163, :345) to direct new shell and self-test sources at the matching `cmake/sources/*.cmake` fragment and Qt Test sources at `crates/pictura-app/cpp/tests/CMakeLists.txt`; verify no remaining instruction to register a shell/self-test source in the root `CMakeLists.txt`

## 4. Verify the pure move

- [x] 4.1 Configure and build with `cmake -S . -B build -G Ninja && cmake --build build --parallel`; verify success
- [x] 4.2 Compare the sorted `"file"` values of `compile_commands.json` against the base revision; verify the two sets are identical
- [x] 4.3 Run `./build/pictura --headless --self-test`; verify no failures
- [x] 4.4 Manual merge check: two branches adding a source to different fragments merge without a conflict in `CMakeLists.txt` or the fragments

## 5. Archive and ship

- [x] 5.1 Run `openspec validate --all --strict`; verify it passes
- [x] 5.2 Archive the change and commit with the issue number; verify `openspec/changes/` holds only `archive/`
- [ ] 5.3 Run `bash scripts/verify-fast.sh`; verify every gate passes (delegated to CI per the task result)
- [x] 5.4 Open the PR with the manual rows as a reviewer checklist; verify CI passes
