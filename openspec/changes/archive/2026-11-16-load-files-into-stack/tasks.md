# Tasks

## 1. Catalogue and bridge scaffolding

- [x] 1.1 Flip `file.scripts.load.files.into.stack` to implemented, add the
  `…` to its label/path leaf, add `FileScriptsLoadFilesIntoStack` to
  `const_names.rs`, add the id to `modal_ids`, regenerate
  `crates/pictura-app/cpp/command_ids.h` and the catalogue goldens, and update
  the catalogue shape counts.
- [x] 1.2 Add `mod stack_files;` to `cxxqt_object.rs` and the bridge file to
  `build.rs`; verify `cargo check -p pictura-app` sees the new bridge.
- [x] 1.3 Create `LoadLayersDialog` with the Source Files group, Use menu, list,
  Browse/Remove/Add Open Files, both checkboxes, and OK/Cancel.
- [x] 1.4 Add the `document_api::loadFilesIntoStack` wrapper and the command
  handler in `frame_commands.cpp`; register the dialog sources and the Qt suite
  in the CMake fragments.

## 2. Stack construction

- [x] 2.1 Write the Rust tests named in `test-plan.md` for canvas sizing, layer
  order, alignment, and smart-object combination; verify they fail only because
  the helpers are absent.
- [x] 2.2 Implement source decoding (raster through `decode_import`, PSD/PSB
  through `read_psd` + `composite_rgba`), the memory gate, layer construction,
  Auto-Align, and document installation.
- [x] 2.3 Add `pictura_render::stack_into_smart_object` and its Rust tests.
- [x] 2.4 Write the Qt cases named in `test-plan.md` for the dialog, source
  selection, stack outcomes, failures, and smart object; verify they pass.
- [x] 2.5 Report unreadable sources after the document is added and refuse the
  run when none decode.

## 3. Records and verification

- [x] 3.1 Add the shipped `Load Files into Stack` line to `ROADMAP.md`.
- [x] 3.2 Run `python3 scripts/check-test-plan.py`, `cargo fmt --all`,
  `cargo clippy -p pictura-app -p pictura-render --all-targets -- -D warnings`,
  `cargo nextest run -p pictura-script -p pictura-app -p pictura-render`, and
  the `tst_load_files_into_stack` suite offscreen.
- [x] 3.3 Archive the change with `openspec archive load-files-into-stack` and
  verify `scripts/check-openspec-archived.sh` and
  `openspec validate --all --strict`.
- [ ] 3.4 Record the manual Browse check in the PR body.
