# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `qts_fixture_single_object` | Build the Qt Test layer; count object files produced for `qt_test_support.cpp`; check each `tst_*` link line for the support library | One `qt_test_support.cpp.o` in the tree, and every suite links `pictura_test_support` | manual: object-file identity and link lines are build-tree observations, recorded in design.md |
| `qts_fixture_state_scope` | Run the suites that assert the state-home restoration over the split fixture | Same result as before the split | `tst_smoke::stateHomeIsolationRestoresPriorValue`, `tst_smoke::sessionWindowGeometryRoundTrips` |
| `qts_fixture_behaviour` | Run the full Qt Test layer | Every registered suite passes, same cases as the base | manual: the whole layer is the check; no single case proves the split changed nothing, so the CTest run is the evidence |
| `qts_fixture_no_moc` | Inspect the support library's compile: no `Q_OBJECT` in `qt_test_support.cpp`, `AUTOMOC OFF` on the target, and no `moc_qt_test_support.cpp` generated | No meta-object is generated for the support unit | manual: `AUTOMOC OFF` is a configure property, read from the generated build files |
| `qts_fixture_no_pch` | Configure with `-DCMAKE_DISABLE_PRECOMPILE_HEADERS=ON` and build the Qt Test layer | Every suite compiles and links | manual: CI configures with precompiled headers disabled and builds the layer; the job is the evidence |
| `qts_fixture_no_frame_h` | Inspect the fixture header and the test tree | No `#include "frame.h"` in the fixture; every window-dereferencing suite includes it or a provider | `check-shell-seams::check_test_fixture_seam` |
| `qts_fixture_seam_guard` | Feed `check_test_fixture_seam` a tree whose fixture includes `frame.h` | The check fails and names the fixture header | `check-shell-seams::self_test` |
| `bp_moc_includes` | Inspect every QObject header and its same-named `.cpp` | Each `.cpp` includes its own `moc_<header>.cpp`; a header without one has a consumer that includes its moc | `check-shell-seams::check_moc_ownership` |
| `bp_moc_incremental` | Edit one of the nine QObject headers and rebuild | Only that header's moc-owning translation unit recompiles | manual: an incremental rebuild observation, recorded in design.md |
| `bp_moc_guard` | Feed `check_moc_ownership` a tree with a same-named `.cpp` that omits its moc include, and one with an unowned header | Both fail, each naming the header and the expected owner | `check-shell-seams::self_test` |
| `bp_bridge_edge_single_owner` | Inspect the fragments and the build script's compiled edge list | No path is both in a fragment and compiled by `cc` | `check-cmake-source-fragments::check` |
| `bp_bridge_edge_tracked` | Edit `decode_image.cpp`, rebuild | The build script reruns and the Rust static library rebuilds | manual: a cargo/Ninja rerun observation, recorded in design.md |
| `bp_bridge_edge_exclusion_exact` | Run the source-list check with an excluded path listed, and with another `.cpp` unlisted | The first fails naming the path, the second fails as unlisted | `check-cmake-source-fragments::self_test` |
| `csf_complete` | Run the source-list check on the tree | No unlisted `.cpp` under `crates/pictura-app/cpp/` outside the exclusion set | `check-cmake-source-fragments::check` |
| `csf_no_duplicates` | Run the source-list check | No duplicate path; the shell list holds the recorded count | `check-cmake-source-fragments::check` |
| `csf_target_owner` | Run the source-list check | Only `main.cpp`/`selftest*` in the self-test fragment | `check-cmake-source-fragments::check` |
| `csf_rust_built_excluded` | Feed the check a fragment that lists an excluded path, and one whose exclusion set covers a non-excluded path | Both fail naming the path | `check-cmake-source-fragments::self_test` |
