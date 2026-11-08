# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `csf_include_order` | Read the root `CMakeLists.txt` after the change | Each fragment's include precedes the target call that consumes its variable (`add_library(pictura_shell ...)` or `add_executable(pictura ...)`) | manual: inspected in the diff; the configure would fail if a variable were empty |
| `csf_root_contract` | Read the root `CMakeLists.txt` after the change | Both targets, the `filter_commands.cpp` property, and `x-release-please-version` remain; no `file(GLOB` over C++ sources and no source paths in the two lists | manual: inspected in the diff (targets, property, marker, glob scope, empty lists) |
| `csf_bridge_stamp` | Configure, then inspect the calls to `pictura_depend_on_bridge_stamp` | Both calls receive the complete shell and self-test lists | manual: the call site is unchanged and the lists are populated by the includes |
| `csf_complete` | Concatenate the `.cpp` paths from `cmake/sources/*.cmake` and compare with `find crates/pictura-app/cpp -type f -name '*.cpp' -not -path '*/tests/*' -not -path '*/panels/actions_tool_recording_bridge.cpp'` | Every translation unit is listed and none is missing | `check-cmake-source-fragments::check` |
| `csf_no_duplicates` | `sort` the concatenated paths and count them | No path appears twice; 556 unique shell paths after the two pre-existing duplicates are removed, and 76 self-test paths | `check-cmake-source-fragments::check` |
| `csf_target_owner` | Read `selftest.cmake` and grep the shell fragments for `selftest` | `selftest.cmake` holds only `main.cpp` and `selftest*`; no shell fragment does | `check-cmake-source-fragments::check` |
| `csf_targets_identical` | Configure the base revision and the change, compare target names | Same target names and dependencies | manual: `cmake --build build --target help` or `ninja -t targets` diff |
| `csf_compile_set_identical` | Extract sorted `"file"` values from both `compile_commands.json` files and `diff` | Identical sets | manual: one-off `diff` recorded in the PR |
| `csf_build_selftest` | `cmake -S . -B build -G Ninja && cmake --build build --parallel`, then `./build/pictura --headless --self-test` | Build succeeds and the self-test reports no failures | manual: commands run locally and in CI |
| `csf_single_fragment` | Add a source to an existing area on a scratch branch and update the build | Only that area's fragment changes; the root `CMakeLists.txt` does not | manual: demonstrated on a scratch branch in the PR |
| `csf_parallel_merge` | Create two branches adding a source to different fragments and merge them | No conflict in `CMakeLists.txt` or the fragments | manual: needs two real branches; reviewer checklist |
| `csf_docs` | Read `AGENTS.md` and `DEVELOPING.md` | Both direct shell/self-test sources to the matching `cmake/sources/*.cmake` fragment and Qt Test sources to `crates/pictura-app/cpp/tests/CMakeLists.txt` | manual: inspected in the diff |
| `csf_guard` | Run `python3 scripts/check-cmake-source-fragments.py --self-test` and the script itself (or `bash scripts/verify-fast.sh`) | The self-test passes and the script passes on the unchanged tree; a planted duplicate, missing listed path, unlisted `.cpp`, or self-test source in a shell fragment fails it | `check-cmake-source-fragments::self_test`, `check-cmake-source-fragments::check` |
