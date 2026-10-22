# Tasks

## Tests

- [x] Rust: `identity_warp_renders_unwarped`, `arc_bends_the_line`, `orientation_and_distortion_differ`, `warped_edges_stay_sharp`, `warp_precedes_transform`, `warp_survives_save_and_open`, `refusals` (pictura-render `type_warp`)
- [x] Rust: `authors_the_warp_descriptor` (pictura-codec `type_write`), `decodes_the_warp_descriptor` (pictura-codec `type_tool`)
- [x] Qt: new `tst_warp_text` suite with every case named in `test-plan.md`, registered in `cpp/tests/CMakeLists.txt`

## Engine

- [x] `pictura-core`: `TextWarp` in `warp.rs`; `TypeSpec::warp`, filled at every construction site
- [x] `pictura-codec`: `decode_type_warp`; `author_type_tool` writes the warp from the spec
- [x] `pictura-render`: `type_warp.rs` (warped outline paths, placement, render, refusal); `font_has_outlines`; `type_placement`, `render_type`, and `type_layer_spec` route the warp; drop the `replace_type_layer` warp `ponytail:`

## App

- [x] Bridge: the warp fields on `TypeSetting`; the new `type_warp.rs` bridge module (refusal, preview, commit); `warped` on the layer-row snapshot
- [x] C++: `warp_text_dialog.{h,cpp}`, a shared warp-style table, Type-menu wiring, the options-bar button, the Layers-panel thumbnail arc, and CMake

## Close-out

- [x] `ROADMAP.md`, `docs/dev/doc-code-parity-gaps.md` (TOOL-17), and `docs/dev/STATE.md`
- [x] `openspec validate --all --strict`, `scripts/check-test-plan.py`, then archive the change
