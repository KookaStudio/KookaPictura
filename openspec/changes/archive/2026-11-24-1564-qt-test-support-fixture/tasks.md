# Tasks

## 1. Checks first (they gate the rest)

- [x] 1.1 Extend `scripts/check-shell-seams.py` with `check_moc_ownership(root)`:
      collect every header under `crates/pictura-app/cpp/` containing `Q_OBJECT`
      or `Q_GADGET`; for a header with a same-named `.cpp`, require that `.cpp`
      to contain `moc_<stem>.cpp`; for a header without one, require some `.cpp`
      under the tree to include it. Report each miss as `<header>: its moc is
      owned by <expected>` / `no translation unit includes moc_<stem>.cpp`. Add
      the failure arms to `self_test()` (a compliant synthetic tree passes; a
      same-named `.cpp` without its moc fails; an unowned header fails).
- [x] 1.2 Add `check_test_fixture_seam(root)`: fail when
      `crates/pictura-app/cpp/tests/qt_test_support.h` contains
      `#include "frame.h"`. Add both arms to `self_test()` (a fixture with the
      include fails; without it passes). Run
      `python3 scripts/check-shell-seams.py --self-test` and confirm it passes.
- [x] 1.3 Teach `scripts/check-cmake-source-fragments.py` the three-path Rust-built
      exclusion set (`decode_image.cpp`, `encode_image.cpp`,
      `panels/actions_tool_recording_bridge.cpp`): replace `RUST_BUILT` with the
      set, keep "not listed" and "listed" both failing, and add self-test cases
      for a listed excluded path and for a widened exclusion hiding a real
      unlisted source. Verify `--self-test` passes.
- [x] 1.4 Add the `check-shell-seams` step to `.github/workflows/guards.yml` so the
      moc and fixture checks run in CI, not only in `scripts/verify-fast.sh`
      (which already runs the script).

## 2. The fixture split

- [x] 2.1 Add `crates/pictura-app/cpp/tests/qt_test_support.cpp`: move the bodies
      of `ScopedStateHome`'s constructor/destructor, `ScopedDefaultLocale`'s
      constructor/destructor, `makeMainWindow`, `completionOffers`,
      `selectFamily`, `typeFamily`, `testFunctions`, `shardTestFunctions`,
      `namesTestFunctions`, `shardArguments`, and `shareRunsNothing` there. The
      file includes `frame.h` (it constructs the window), `qt_test_support.h`,
      and the Qt headers the bodies use.
- [x] 2.2 Reduce `crates/pictura-app/cpp/tests/qt_test_support.h` to declarations:
      forward-declare `class PicturaMainWindow;` in the `pictura` namespace, drop
      `#include "frame.h"`, declare the moved functions, and inline the class
      member declarations. Keep `shardedMain` as a template definition and keep
      only the Qt headers the declarations need. Verify
      `python3 scripts/check-shell-seams.py` reports no fixture-seam failure.
- [x] 2.3 Add the `pictura_test_support` STATIC library in
      `crates/pictura-app/cpp/tests/CMakeLists.txt` with `AUTOMOC OFF`, linking
      `pictura_shell` and `Qt6::Test` exactly as `pictura_test_pch` does, with the
      same `${PICTURA_DEV_COMPILE_OPTIONS}` and `-g1`, and link it PRIVATE into
      every `tst_*` target. Leave `pictura_test_pch` and its `REUSE_FROM` wiring
      untouched. When `BUILD_TESTING` is off the target does not exist, so guard
      the block like the PCH one.
- [x] 2.4 Add `#include "frame.h"` to the 16 suites that name the window, call the
      factory, and reach `frame.h` only through the fixture. The list measured on
      the base: `tst_app_updater`, `tst_document_registry`, `tst_fonts`,
      `tst_history_log`, `tst_import_pdf`, `tst_measurement_scale`,
      `tst_preferences_store`, `tst_preset_manager`, `tst_psd_compatibility`,
      `tst_save_pdf`, `tst_structured_logging`, `tst_swatches_panel`,
      `tst_tool_options_persistence`, `tst_ui_localization`, `tst_untrusted_input`,
      `tst_view_menu`. Eight other suites leave the set by no longer getting
      `frame.h` from the fixture: `tst_color_picker`, `tst_color_settings`,
      `tst_document_api`, `tst_import_save_coordinator`, `tst_panel_host`,
      `tst_preferences_reset`, `tst_settings_backup`, `tst_webp`. Re-derive both
      lists after 2.1–2.3 with the header-graph walk in `design.md` rather than
      trusting this snapshot, and add any suite that appears once the fixture
      stops providing the definition. The expected outcome is 242 of 269 suites
      parsing the closure.
- [x] 2.5 Route `multi_file_test_support.h`'s `frame.h` include: the header needs
      the window definition for its helpers, so keep it there and confirm its four
      consumers (`tst_multi_file_open`, `tst_place_sequence`,
      `tst_smart_object_relink`, `tst_command_tree`) are unaffected.

## 3. The nine moc owners

- [x] 3.1 Append `#include "moc_<header>.cpp"` after the closing
      `} // namespace pictura` line of each: `fade_dialog.cpp`,
      `import_save_coordinator.cpp`, `load_layers_dialog.cpp`, `panel_host.cpp`,
      `save_for_web_color_table.cpp`, `save_load_selection_dialog.cpp`,
      `save_optimized_dialog.cpp`, `statistics_dialog.cpp`,
      `system_info_dialog.cpp`. Match the 175 compliant files' shape exactly (blank
      line, then the include).
- [x] 3.2 Verify with `python3 scripts/check-shell-seams.py` that
      `check_moc_ownership` reports no miss, and that after the build
      `build/crates/pictura-app/pictura_shell_autogen/mocs_compilation.cpp`
      contains no `#include "moc_` line for the nine. Record the generated unit's
      moc include count (expected 0) in `design.md`'s Measurements section.

## 4. One owner per bridge edge

- [x] 4.1 Remove `crates/pictura-app/cpp/decode_image.cpp` and
      `crates/pictura-app/cpp/encode_image.cpp` from
      `cmake/sources/documents.cmake`, keeping their `.h` lines only if the header
      is still useful for IDE grouping (it is a bridge edge header, so drop both
      the `.cpp` and `.h` lines and let `cc` own the pair). Keep
      `panels/actions_tool_recording_bridge.h` in `panels.cmake`.
- [x] 4.2 Run `python3 scripts/check-cmake-source-fragments.py` and confirm it
      passes with the three-path exclusion set. Verify
      `cargo nextest run -p pictura-app` still links (the bridge symbols resolve
      from `cc`'s objects in the static library); if it does not, restore the two
      CMake entries, drop the `cc.file(...)` lines instead, and record the reversal
      in `design.md`.
- [x] 4.3 Confirm the build script still tracks all six `BRIDGE_EDGES` entries
      (`grep -c 'cpp/decode_image\|cpp/encode_image' crates/pictura-app/build.rs`
      is unchanged) so an edge edit still reruns it.

## 5. Build, verify, measure

- [x] 5.1 One CMake build through the wrapper:
      `bash scripts/with-build-lock.sh -- bash -c 'cmake -S . -B build -G Ninja && cmake --build build --parallel'`.
- [x] 5.2 Run `bash scripts/verify-full.sh` (the change touches the C++ shell).
      Fix failures; re-run only what changed. Note any environment-only failure
      (the present-cache GPU row and `LC_ALL`) separately rather than weakening a
      test.
- [x] 5.3 Take every "after" number in `design.md`'s Measurements table with the
      same command as the "before" column, fill the table, and spell out each
      delta. Counts are counts; if a time is taken from `build/.ninja_log`, label
      it a time and say the machine.
- [x] 5.4 Confirm the acceptance that is reachable: fixture header has no
      `frame.h` include; the nine moc edges present and `mocs_compilation.cpp`
      gains no include; the two edges compile in one place; no new
      `scripts/file-size-allowlist.txt` entry. Record the unreachable one (0 test
      TUs including `frame.h`) with the 241 figure and the #1451 dependency.

## 6. Archive and ship

- [x] 6.1 `openspec archive 1564-qt-test-support-fixture`, then
      `bash scripts/check-openspec-archived.sh`, `openspec validate --all
      --strict`, and `python3 scripts/check-test-plan.py`.
- [x] 6.2 Review `ROADMAP.md` and `PARITY.md`: this is build tooling with no
      feature coverage change, so update them only if a tracked build item names
      this work.
- [x] 6.3 Commit in concern order — checks, fixture split, moc owners, bridge
      edge owner, measurements — each ending `(#1564)`. Push, open the PR with the
      conventional title and the measurement table, list the `manual:` plan rows
      as the reviewer checklist, and state the narrowed scope. Do not merge.
