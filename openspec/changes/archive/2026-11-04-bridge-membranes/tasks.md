# Tasks

## 1. Spike and membrane plan

- [x] 1.1 Script the consumer co-inclusion table for the 82 generated headers (81 consumed) and record the chosen membrane -> header mapping in `design.md`; verify every consumed generated header is reachable through exactly one membrane or adapter header, `type_tools*.cxxqt.h` are owned by `cpp/api/type_api.h`, and `impl_paint/tool_recording.cxxqt.h` is exempt (no consumer). No membrane includes all of them.
- [x] 1.2 List the bridges that expose at least one `extern "Rust"`/`extern "RustQt"` function and two or fewer, and their target neighbours; verify each target declares more than two functions after the proposed move, no moved struct or function name collides, each moved item keeps its namespace, `decode_bridge::copy_bytes` stays unnamespaced (or its four call sites are updated), each bridge's `imp` module is re-homed, and each target stays under the 1,204-line cap.

## 2. Membranes and seam tests

- [x] 2.1 Add `bridge_headers_confined_to_api`, `bridge_membranes_are_include_only`, `no_universal_bridge_membrane`, `no_small_bridge_files`, `generated_include_scan_rejects_violation`, `adapter_and_membrane_headers_are_registered`, and `bridge_count_within_budget` to `crates/pictura-app/src/build_config.rs`; make the membrane tests assert the membrane set is non-empty so they cannot pass vacuously. `bridge_headers_confined_to_api` turns green in group 4, and `no_small_bridge_files` and `bridge_count_within_budget` in group 5; the rest are green by the end of this group.
- [x] 2.2 Create the `cpp/api/<domain>_bridge.h` membranes from the 1.1 mapping; verify each consumed generated header is reachable through exactly one membrane or adapter header and `cargo nextest run -p pictura-app` passes.
- [x] 2.3 Register the membranes in `CMakeLists.txt` and verify `cmake --build build` succeeds and CMake tracks them.

## 3. Migrate production includes

- [x] 3.1 Replace direct generated includes in `cpp/*.cpp`, `cpp/*.h`, `cpp/panels/`, `cpp/acr/`, and `cpp/tool*.cpp` with the owning membrane(s) or adapter header; verify `rg '\.cxxqt\.h' cpp --glob '!**/api/**' --glob '!**/tests/**'` is empty and the build succeeds.
- [x] 3.2 Migrate `cpp/frame_includes.h` and verify the build and `tst_smoke` pass.
- [x] 3.3 Run `cargo nextest run -p pictura-app` and verify `bridge_headers_confined_to_api` and `adapter_and_membrane_headers_are_registered` pass.

## 4. Migrate test includes

- [x] 4.1 Replace generated includes in `cpp/tests/*.cpp` and `cpp/tests/*.h` (for example `tst_command_tree_cases.h`) with membranes; verify no test file includes `*.cxxqt.h` and `ctest --test-dir build -R '^tst_' --output-on-failure` passes.
- [x] 4.2 Run `bridge_headers_confined_to_api` again and verify it scans the test tree with no violations.

## 5. Fold small bridges

- [x] 5.1 Move declarations from the small bridges into their neighbours, re-home each bridge's `imp` module (or keep the file as a plain, non-bridge module) and add the `mod`/`use` wiring, delete the bridge files, and update `build.rs` `.files([...])`; verify `no_small_bridge_files` and `bridge_count_within_budget` pass and `cargo nextest run -p pictura-app` passes.
- [x] 5.2 Update the membranes for the removed headers; verify the confinement and universal-membrane tests still pass.
- [x] 5.3 Run `cmake --build build`, then `ctest -R '^tst_'` and both `./build/pictura --headless --self-test` runs; verify every exit code is unchanged.

## 6. Bridge stamp and rebuild behavior

- [x] 6.1 Rewrite `pictura_depend_on_bridge_stamp` to attach the stamp to every shell and test translation unit; verify a bridge edit recompiles every membrane and adapter consumer in one build and a no-op build recompiles none, recording the Ninja counts in `design.md`. Rebuild fan-out stays global.
- [ ] 6.2 Verify `bp_bridge_impl_edit` and `bp_bridge_stamp_fallback` still hold by editing an implementation body and a malformed bridge module on a scratch branch; record both measurements in `design.md`.

## 7. Integration and docs

- [x] 7.1 Update `docs/dev/refactor-view-seam.md` and `docs/dev/STATE.md` with the membrane decision, the `type_api.h` exception, the new tests, the global rebuild fan-out, and the deferred verb manifest; verify the docs describe the shipped state. Commit docs with `TASK-ALLOWS-DOCS`.
- [x] 7.2 Opened follow-up issue #1187 for the deferred verb manifest; state in the PR body that this is a deliberate narrowing of issue #1160 and needs maintainer sign-off. Do not add the manifest.
- [x] 7.3 Archive the change with `openspec archive bridge-membranes` before the final gate; verify `scripts/check-openspec-archived.sh` passes.
- [ ] 7.4 Run `bash scripts/verify-full.sh` and `openspec validate --all --strict`; verify both pass.
