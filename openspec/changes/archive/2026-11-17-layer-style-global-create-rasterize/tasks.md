# Tasks

## 1. Global light model and PSD resources

- [x] 1.1 Add `pictura_core::GlobalLight` (angle/altitude, default 30/30) in a new `global_light` module and a `global_light` field on `Document`; verify `cargo test -p pictura-core` and that `scripts/check-file-size.sh` keeps `crates/pictura-core/src/lib.rs` under 1200 lines
- [x] 1.2 Add `pictura-codec` global-light read/resolve (resources 1037 angle, 1049 altitude, four-byte big-endian signed integers, default 30 per missing value) and unit tests for both-resources, missing-resource and malformed-resource cases; verify `cargo test -p pictura-codec global_light`
- [x] 1.3 Add the write/sync path (`with_document_global_light`) and wrap the `guide_resources::output_resources` call in `write.rs` with it; when either value differs from the stored pair, write both resources, and when both match leave the section untouched; verify `write_psd` tests for edited, untouched-absent, and matching-resource sections, with unrelated resources preserved
- [x] 1.4 Call the resolver from both open paths (`read.rs`, `lazy.rs`) and verify the full and lazy read tests still pass

## 2. Render global light

- [x] 2.1 Add `layer_effects::light::effective`, apply it in `build_layer_passes`, and verify shadow-angle, local-flag, and inner-shadow angle tests
- [x] 2.2 Add the document light to `pass_key_hash` and make `paint_reach` conservative for global-angle shadows; verify a composite-change test proves the cached passes are rebuilt and a unit assertion shows `paint_reach` is at least `ceil(distance·√2)` for a globally-lit shadow
- [x] 2.3 Make Bevel & Emboss take the global altitude, verify a bevel pixel test at two altitudes and a global-angle bevel test, and update the `layer_effects` module ceiling comments that say the global light is not decoded
- [x] 2.4 Audit and update existing tests that build `uglg=true`: `crates/pictura-render/src/tests/layer_effects.rs`'s `ShadowSpec` helper, styles authored through `set_layer_style_value` (whose `useGlobalLight` defaults on), and `layer_style_tests::drop_shadow_authors_a_decodable_effect_that_renders`; set the flag false (or the document light) where a local-angle assertion is intended, and verify the render suites

## 3. Global Light command and dialog

- [x] 3.1 Add `set_document_global_light` (normalise angle to 0–359, clamp altitude to 0–90, false when unchanged) with unit tests; verify `cargo test -p pictura-render global_light`
- [x] 3.2 Add the bridge getters/setter that record one "Global Light" state with `Edit::Full`, register the `Layer ▸ Layer Style ▸ Global Light…` handler with a document-open enabled provider, and remove the stale “Layer Style … stay disabled” comment in `menu_registrar.cpp` and the `menu_layer_style.cpp` ponytail note
- [x] 3.3 Add `global_light_dialog.{h,cpp}` (Angle 0–359, Altitude 0–90), register both files in `cmake/sources/dialogs.cmake`, and run the dialog under `runDialog`
- [x] 3.4 Disable and feed the effect pages' Angle (and Bevel Altitude) controls from the document light while Use Global Light is checked, restoring the stored local value when unchecked
- [x] 3.5 Point the panel's layer-style submenu Global Light… entry at the registry action and update `tst_layers_effects::rowMenuListsLayerStyleCommands` to expect it enabled
- [x] 3.6 Add Qt tests for the dialog (one state, cancel records none, undo restores), the angle field and the bevel altitude field; update `tst_layer_style::menuCommands` to expect Global Light enabled; verify `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_(layer_style|layers_effects)$' --output-on-failure`
- [x] 3.7 Flip `layer.layer.style.global.light` to `implemented: true` in `entries.rs`, update `tests/golden/catalogue.tsv`, and verify `cargo test -p pictura-script catalogue`

## 4. Rasterize Layer Style

- [x] 4.1 Add `LayerRasterizeLayerStyle` to `const_names.rs`, regenerate `command_ids.h`, update `tests/golden/const_names.tsv`, and verify `python3 scripts/gen-command-ids.py --check` plus the `pictura-script` catalogue tests
- [x] 4.2 Add `can_rasterize_layer_style` / `rasterize_layer_style` with the corrected scratch recipe (copy `global_light`, `layer_section_extra`, `is_psb`; neutralise vector mask/Blend If/knockout; keep fill while baking then set the result fill to 255) and the full refusal list (unstyled, group, Background, non-raster, no enabled and present effect, master switch off, customized Blend If, non-None knockout, base owning a clipped run); verify Rust tests for bake-and-clear, Fill 50 %, global light, pattern overlay, vector mask, and each refusal
- [x] 4.3 Add the bridge command and register the Layer ▸ Rasterize ▸ Layer Style handler with its enabled provider; verify the command dispatches and records one "Rasterize Layer Style" state in Qt
- [x] 4.4 Add the `RasterizeStyle` row-action dynamic case and its styled-raster fact, add `rasterizeStyle` to the row-menu text-override list, and update `tst_layers_row_model::unimplementedRowsAreDisabled` to stop using it as the disabled exemplar; verify `tst_layers_row_model` expects the command id and enabled state
- [x] 4.5 Update `tst_layers_row_menu::rasterizeRows` and `unimplementedDisabled` (styled pixel row enabled, styled fill row disabled, no not-implemented tooltip on the style row) and verify the suite
- [x] 4.6 Flip `layer.rasterize.layer.style` to `implemented: true`, update the catalogue golden, and verify the catalogue test

## 5. Create Layers

- [x] 5.1 Add `layer_effects/materialize.rs` (a `pub(crate)` helper) to apply `light::effective` and materialise each enabled family's passes into per-layer RGBA buffers, with one layer per pass (two for most bevel styles, four for Pillow); verify a materialisation test
- [x] 5.2 Add `can_create_style_layers` / `create_style_layers` with increasing-offset above-pass insertion, blend/base-opacity/opaque-fill/no-mask inheritance, and the full refusal list (including a clipped base and a base owning a clipped run); verify Rust tests for recompose equality, inheritance, above-order (≥3 effects), global light, Pillow pass count, and refusals
- [x] 5.3 Add the bridge command and register the `Layer ▸ Layer Style ▸ Create Layers` handler with its enabled provider; verify one "Create Layers" state and undo
- [x] 5.4 Point the panel submenu Create Layers entry at the registry action, unify its label, and update `tst_layers_effects::rowMenuListsLayerStyleCommands` to expect it enabled and labelled "Create Layers"
- [x] 5.5 Add Qt tests for the command (created layers, base unstyled, one state, undo) and the panel submenu; verify `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_(layer_style|layers_effects|layers_row_model|layers_row_menu)$' --output-on-failure`
- [x] 5.6 Flip `layer.layer.style.create.layers` to `implemented: true`, update the catalogue golden, and verify the catalogue test

## 6. Docs and integration

- [x] 6.1 Tick `ROADMAP.md` for #580; mark `LAY-2` SHIPPED in `docs/dev/doc-code-parity-gaps.md` and fix its stale `frame_menus_layer_style.cpp` path; correct `docs/05-layers/layer-styles.md` (Create Layers uses plain layers, not a clipping group; the pair is 1037 angle + 1049 altitude at both occurrences; Rasterize lives under Layer ▸ Rasterize), `docs/01-architecture/document-model.md:99`, and the stale disabled-stub note in `docs/dev/STATE.md`; record `TASK-ALLOWS-DOCS`
- [x] 6.2 Fill every coverage cell in `test-plan.md` and verify `python3 scripts/check-test-plan.py` and `openspec validate layer-style-global-create-rasterize --strict`
- [ ] 6.3 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo nextest run --workspace`
- [ ] 6.4 Build the app and run `bash scripts/verify-fast.sh`, then archive the change with `openspec archive layer-style-global-create-rasterize` in the shipping PR
