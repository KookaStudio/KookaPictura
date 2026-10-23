# Tasks

## Tests

- [x] Rust: `curve_maps_coverage`, `type_edges_thin_by_default`, `opacity_passes_through_the_curve`, `pixel_layers_skip_the_curve`, `materialize_bakes_the_curve`, `gpu_defers_type_while_gamma_is_on`, `replace_type_fonts_replaces_named_fonts` (pictura-render units); `off_reproduces_the_plain_blend` (its own integration-test binary)
- [x] Qt: new `tst_text_gamma_fonts` suite with every case named in `test-plan.md`, registered in `cpp/tests/CMakeLists.txt`

## Engine

- [x] `pictura-render`: `text_gamma.rs` (setting + curve); the curve in `blend_into_row` for type layers; the GPU refusal; the baked curve in `materialize_text_rgba`
- [x] `pictura-render`: `missing_fonts.rs` with `replace_type_fonts`

## App

- [x] Bridge: `type_tools/fonts.rs` (set gamma, recomposite, replace fonts); `missing_font` on the layer-row snapshot is computed in C++
- [x] C++: Color Settings More Options; session keys; startup apply; `fontIsInstalled`; open-time alert; Type menu entry; Layers-row badge; CMake

## Close-out

- [x] `ROADMAP.md`, `docs/dev/doc-code-parity-gaps.md` (TOOL-17), and `docs/dev/STATE.md`
- [x] `openspec validate --all --strict`, `scripts/check-test-plan.py`, then archive the change
