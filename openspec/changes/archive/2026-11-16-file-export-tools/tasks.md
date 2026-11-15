# Tasks

## 1. Catalogue and shell scaffolding

- [x] 1.1 Add `file.export.paths.to.illustrator` to
  `crates/pictura-script/src/command_catalogue/entries.rs`, add the `…` to the
  Export Layers label and path leaf, flip `implemented` on the Zoomify and
  Export Layers rows, add the constants the handlers use to
  `crates/pictura-script/src/command_catalogue/const_names.rs`, update the
  hard-coded 662/434/358 counts in `tests/catalogue.rs`, and regenerate
  `crates/pictura-app/cpp/command_ids.h` and
  `crates/pictura-script/tests/golden/catalogue.tsv`; verify
  `cargo test -p pictura-script` passes.
- [x] 1.2 Create the dialog headers/sources
  (`zoomify_dialog`, `export_paths_dialog`, `export_layers_dialog`) as empty
  classes and the two Qt suite sources with the case declarations named in
  `test-plan.md`; verify the files exist and the suite case names match the
  plan.
- [x] 1.3 Register the dialog sources in the matching `cmake/sources/*.cmake`
  fragment and the two suites in `crates/pictura-app/cpp/tests/CMakeLists.txt`;
  verify `python3 scripts/check-cmake-source-fragments.py` passes and the
  CMake configure lists the new targets.
- [x] 1.4 Add the three ids to `modal_ids` and register
  `setEnabledProvider` handlers so Zoomify, Paths, and Export Layers are
  disabled without a document; verify `tst_file_export_tools`'s
  `exportCommandsNeedADocument` is planned and the modal list includes all
  three.
- [x] 1.5 Add a test-only export delay/gate (like `save_set_delay`) to the
  export job path so a batch can be held past 250 ms; verify the Zoomify and
  Export Layers cancel cases can use it.

## 2. Zoomify export

- [x] 2.1 Write the Rust unit tests named in `test-plan.md` under
  `pictura-codec` (tier geometry, group rollover, edge tiles, XML, viewer
  self-containment); verify they run with
  `cargo test -p pictura-codec zoomify_` and fail only because the module is
  absent.
- [x] 2.2 Implement `pictura-codec/src/zoomify.rs` (`plan`,
  `image_properties`, `viewer_html`) with no `fetch`/XHR/import in the viewer,
  and verify `cargo test -p pictura-codec zoomify_` passes.
- [x] 2.3 Write the Zoomify cases named in `test-plan.md`, including the
  dialog defaults/bounds, written tile tree/XML for each geometry scenario,
  white tolerance, offline HTML, cancel, and unchanged document; verify the
  suite is registered and the cases are listed by the build.
- [x] 2.4 Implement `ZoomifyDialog`, the command handler, base-size render,
  tier downscale, tile JPEG encode with white flattening, output-folder checks,
  progress, and cancel; verify the Zoomify cases pass.
- [ ] 2.5 Record the manual browser pan/zoom check for the PR.

## 3. Paths to Illustrator

- [x] 3.1 Write the Rust unit tests named in `test-plan.md` for the EPS and
  PDF writers (all paths, ASCII names, open/closed subpaths, handles, page
  size, headers including `%%Pages: 1`, stroke, determinism, no raster); verify
  `cargo test -p pictura-codec illustrator_` runs and fails only because the
  module is absent.
- [x] 3.2 Implement `pictura-codec/src/illustrator.rs` with the shared path
  walk, ASCII name escaper, coordinate formatter, `write_eps`, and `write_pdf`;
  verify `cargo test -p pictura-codec illustrator_` passes.
- [x] 3.3 Write the Paths cases named in `test-plan.md`, including the `.ai`
  suffix, no-path refusal, unwritable parent, and unchanged document; verify
  the build lists them.
- [x] 3.4 Implement `ExportPathsDialog`, the command handler, the two format
  choices, the `.ai` suffix, the no-path refusal, and the atomic `QSaveFile`
  write; verify the Paths cases in `tst_file_export_tools` pass.
- [ ] 3.5 Record the manual Illustrator check for both flavours for the PR
  body.

## 4. Export Layers to Files

- [x] 4.1 Write the Rust unit tests named in `test-plan.md` for target
  enumeration, ancestor visibility, adjustment targets, name sanitization,
  collisions, empty names, and UTF-8 truncation, plus a scratch-document test
  that the layer keeps its original rect and the composite is rebuilt; verify
  `cargo test -p pictura-app export_layers_` runs and fails only because the
  module is absent.
- [x] 4.2 Implement the target enumerator, namer, scratch-document bridge
  (copy `width`, `height`, `mode`, `depth`, `source_mode`, `document_icc`, and
  `image_resources`; force the target visible; rebuild `composite` and
  `merged_composite_present`), and the working-space document-sized composite;
  verify `cargo test -p pictura-app export_layers_` passes.
- [x] 4.3 Write the Export Layers cases named in `test-plan.md`, including the
  defaults, hidden-layer non-empty output, working-space ICC pixels, CMYK
  source, PDF physical page and editing payload, PSD reopen, existing-file
  replacement, directory failure, cancel, and unchanged document; verify the
  suite is registered and its cases are listed by the build.
- [x] 4.4 Implement `ExportLayersDialog` and the batch: PSD/JPEG/TIFF/BMP/PDF
  serialization, white flattening, ICC control for PSD/JPEG/TIFF/PDF, clear the
  profile when off, destination checks, replace-existing-file semantics,
  directory failure, progress, and cancel; verify
  `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_export_layers$' --output-on-failure`
  passes.

## 5. Integration and records

- [x] 5.1 Run the targeted checks and fix any failure (deferred until the
  maintainer asks): `cargo fmt --all`,
  `cargo clippy -p pictura-codec -p pictura-app --all-targets -- -D warnings`,
  `cargo nextest run -p pictura-script -p pictura-codec -p pictura-app`,
  `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R 'tst_(file_export_tools|export_layers|busy_job)' --output-on-failure`,
  and `python3 scripts/check-test-plan.py`.
- [x] 5.2 Update `ROADMAP.md` (tick Zoomify and Paths to Illustrator; mark
  Export Layers PARTIAL and keep #587 open for the shared option surface and
  Targa; leave Data Sets As Files on #622 and Render Video as deferred
  Extended-only), `docs/dev/doc-code-parity-gaps.md` rows WF-6 and LAY-9, and
  `docs/02-ui-ux/menus.md`'s Export line; verify `openspec validate --all
  --strict` passes and mark the docs commit `TASK-ALLOWS-DOCS`.
- [x] 5.3 Archive the change with `openspec archive file-export-tools` and
  verify `bash scripts/check-openspec-archived.sh` passes and the main specs
  (including `document/long-operation-progress`) are updated under
  `openspec/specs/`.
- [ ] 5.4 Run `bash scripts/verify-fast.sh` and record the two manual plan rows
  (Illustrator opens both flavours; the Zoomify page pans and zooms) in the PR
  body. Issue disposition: the core of #587 ships but #587 stays open for the
  shared option surface and Targa; #630 stays open for Data Sets and Render
  Video.
