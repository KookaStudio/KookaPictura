# Tasks

## 1. Typed adjustment kind in `pictura-core`

- [x] 1.1 Generate `AdjustmentKind` and `ALL` from one declaration, with `name()`, `from_name()`, and unique round-trip coverage.
- [x] 1.2 Add exhaustive `Adjustment::kind()` and `FillContent::kind()` conversions. Auto fans out to its three kinds; Replace Color and Match Color return `None`.
- [x] 1.3 Replace the core direct-kind match with six const direct-adjustment constructors for catalogue rows to reference.

## 2. Default-block builders in `pictura-codec`

- [x] 2.1 Replace the kind-keyed dialog and layer matches with per-kind builder hooks while preserving every payload byte. Compare representative complete payloads for the former arms.
- [x] 2.2 Keep the CMYK Channel Mixer identity and its dedicated test.
- [x] 2.3 Keep render's `dialog_default` and `layer_default` convenience wrappers over the catalogue rows.

## 3. Catalogue rows carry every fact

- [x] 3.1 Add `dialog_default`, `layer_default`, and `direct` function-pointer fields plus `gpu: GpuRoute`. Compare the catalogue bidirectionally with generated `ALL` and remove fixed taxonomy lengths.
- [x] 3.2 Change `GpuRoute::Inline` to carry only `id`. Add the GPU-feature `catalogue_rows_keep_every_fact_in_sync` test in `gpu/route_tests.rs`, covering every row's defaults, decoded kind/title, direct adjustment, route eligibility, derived WGSL markers, LUT shape ids, and parity-source synchronization.
- [x] 3.3 Keep row-iteration tests for dialog defaults, layer defaults, and deliberate dialog/layer divergence.

## 4. GPU planning and parity follow the rows

- [x] 4.1 Resolve inline and LUT routes through the catalogue and replace `inline_params`'s wildcard with explicit unmodelled adjustment groups.
- [x] 4.2 Obtain LUT shape ids from `gpu::lut::kind` and include them in id uniqueness and WGSL checks.
- [x] 4.3 Extend `gpu_route_falls_back_for_unmodelled_parameters` to assert both GPU planner refusal and an applied CPU fallback result.
- [x] 4.4 Make `adjustment_layers_match_cpu` require a layer default for every routed row and assert `checked == expected`.

## 5. Shell consumers keep their APIs

- [x] 5.1 Build app adjustment layers through render's layer-default wrapper.
- [x] 5.2 Resolve direct Image commands through render's row-backed `direct_adjustment` wrapper and take the history label from the row title.
- [x] 5.3 Keep one decoded-block kind mapping. Assert that Auto kinds map to typed catalogue values but have no preset store or save whitelist entry, and that Replace Color and Match Color stay outside the catalogue.
- [x] 5.4 Add same-kind replacement guard tests for equal, different, and undecodable blocks.
- [x] 5.5 Drive the control server's adjustment list from catalogue dialog rows and keep the Qt coverage at sixteen dialog kinds.

## 6. Color Lookup default

- [x] 6.1 Point the Color Lookup dialog and layer rows at the same `None` preset builder.
- [x] 6.2 Keep the app test that resolves a newly created Color Lookup layer to preset index 0.

## 7. Documentation and verification

- [x] 7.1 Update `docs/dev/STATE.md` with the row-backed seam and the test's `gpu/route_tests.rs` path. Any commit containing this docs change must carry `TASK-ALLOWS-DOCS` in its subject.
- [x] 7.2 Run `cargo fmt --all` and `bash scripts/with-build-lock.sh -- cargo clippy --workspace --all-targets -- -D warnings`.
- [x] 7.3 Run `bash scripts/with-build-lock.sh -- cargo nextest run --workspace`. The final `verify-fast.sh` run completed this with 6,802 passed and 136 skipped before hitting a proven baseline Qt failure.
- [x] 7.4 Run `bash scripts/with-build-lock.sh -- env QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_(adjustments_panel|command_tree|image_adjustments|control_editing)$' --output-on-failure` and confirm the 19-entry panel menu and existing command behavior.
- [x] 7.5 Run `openspec validate adjustment-facts-single-source --strict` and `python3 scripts/check-test-plan.py`.
- [x] 7.6 Run `bash scripts/with-build-lock.sh -- bash scripts/check-file-size.sh`.
