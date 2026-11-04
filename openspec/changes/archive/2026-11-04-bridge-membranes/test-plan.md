# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A panel includes only adapter headers | Inspect every C++ TU outside `cpp/api/` for generated includes | No generated header is included | `pictura-app::bridge_headers_confined_to_api` |
| Adapter registration is explicit | Read `CMakeLists.txt` and list every `cpp/api/*.h` | Every adapter and membrane is listed | `pictura-app::adapter_and_membrane_headers_are_registered` |
| Pass-through adapters are rejected at review | Review a proposed adapter against the batching/ordering/typed-protocol test | A 1:1 forwarder is not added; callers use the membrane | manual: review judgment, not mechanically testable |
| Uncovered operations are named deferrals | Search the tree for generated-header deferrals outside `cpp/api/` | No generated header is included outside `cpp/api/` | `pictura-app::bridge_headers_confined_to_api` |
| Generated headers are confined to cpp/api | Scan `cpp/`, including `cpp/tests/`, production headers, and `frame_includes.h`, for `*.cxxqt.h` | No include outside `cpp/api/` (type bridge headers remain governed by the existing type-adapter requirement) | `pictura-app::bridge_headers_confined_to_api` |
| A membrane carries no logic | Read each `cpp/api/*_bridge.h` | Only `#pragma once`, includes, and comments, no bodies or state | `pictura-app::bridge_membranes_are_include_only` |
| No membrane is a universal header | Read each membrane's include set | No membrane includes every generated header; each is a proper subset of the generated headers | `pictura-app::no_universal_bridge_membrane` |
| The small bridges are gone | Count exposed `extern "Rust"`/`extern "RustQt"` functions per bridge file | Every bridge with at least one exposed function declares more than two; import-only bridges are exempt; bridge count `<= 69` | `pictura-app::no_small_bridge_files` |
| A folded verb keeps its call sites working | Build and run the Qt suites and both self-tests after a fold | Same generated symbol; suites and exit codes unchanged | `tst_adjustments_panel::gridShowsAllNineteenInOrder`, `tst_hdr_conversion::acceptConvertsWithSelectedMethod`, `tst_export_as::svgExport`, `tst_new_document::createsEachModeWithOneState`, `tst_nested_layer_target::pathBridgesResolveTheNestedLeaf`, `tst_command_tree::menus` |
| A new direct generated include fails the gate | Feed the scanner a synthetic tree with a direct include | The scanner reports the offending file | `pictura-app::generated_include_scan_rejects_violation` |
| Bridge growth fails the gate | Count bridge files and check `cxxqt_object.rs` size | Bridge count `<= 69`; file `<= 1204` lines | `pictura-app::bridge_count_within_budget` |
| bp_bridge_same_build | Change a bridge module, build once, build again | Every membrane and adapter consumer recompiles in the first build; none in the second | manual: Ninja rebuild measurement (recorded in `design.md`) |
| bp_bridge_no_change | Rebuild with no source change | No C++ TU recompiles | manual: Ninja rebuild measurement (recorded in `design.md`) |
| bp_bridge_impl_edit | Edit a body outside the bridge module, rebuild | Stamp unchanged; no C++ TU recompiles | manual: Ninja rebuild measurement (recorded in `design.md`) |
| bp_bridge_stamp_fallback | Make a bridge module unparsable, rebuild | The whole file is hashed and dependents rebuild | manual: Ninja rebuild measurement (recorded in `design.md`) |
