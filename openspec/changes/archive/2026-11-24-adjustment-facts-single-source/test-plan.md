# Test plan

Covers every scenario in `imaging/adjustment-catalogue` and
`imaging/adjustment-layers`. Coverage names the test that asserts each scenario.
The focused acceptance run and full workspace Rust run passed on 2026-11-24. The
final aggregate stopped on a GPU pixel test that fails identically on clean
`origin/master`; the verification record gives the reproduction and remaining
guard results.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `adjust_catalogue_complete` | Enumerate `ADJUSTMENT_CATALOGUE`, assert unique kinds, dialog order, one row per `AdjustmentKind::ALL` value, and a dialog builder for every dialog row | Every kind appears once, sixteen dialogs stay in CS6 order, and each dialog block decodes under its row title | `pictura-render::adjustment_catalogue_covers_every_kind_once` |
| `adjust_catalogue_layer_rows` | Filter rows to adjustment-layer entries and compare kind order and ellipsis flags | Sixteen layer kinds stay in catalogue order, and only Invert lacks an ellipsis | `pictura-render::adjustment_catalogue_layer_entries` |
| `adjust_catalogue_commands` | Look up each row's Image path and layer path in the command catalogue | Every path resolves to a command id, and Auto rows resolve at the Image root | `pictura-app::adjustment_catalogue_rows_resolve_commands` |
| `adjust_facts_dialog_default` | Invoke each dialog row's builder and open the editor on its block | The block decodes under the row title, and every editor parameter starts at its declared default | `pictura-render::adjustment_row_dialog_defaults` |
| `adjust_facts_layer_default` | Invoke each adjustment-layer row's builder, decode the block, and open its editor | The block decodes to the row's typed kind and opens under the row title | `pictura-render::adjustment_row_layer_defaults`, `pictura-render::catalogue_rows_keep_every_fact_in_sync` |
| `adjust_facts_layer_default_cmyk` | Build the Channel Mixer row's default for RGB and CMYK documents | The CMYK block carries 100-percent ink rows and zero ink constants, and differs from RGB | `pictura-codec::cmyk_channel_mixer_layer_default` |
| `adjust_facts_direct` | Invoke every row's direct constructor and compare the resulting adjustment kind with the row | Direct rows yield an adjustment of their own kind, and every other row yields none | `pictura-render::catalogue_rows_keep_every_fact_in_sync` |
| `adjust_facts_gpu_id` | Derive `kind == {id}u` from every inline row id and both ids returned by `gpu::lut::kind`, then scan only the shader's adjustment dispatch | Ids 1 through 7 are unique, and each derived marker occurs exactly once | `pictura-render::catalogue_rows_keep_every_fact_in_sync` |
| `adjust_facts_kind_map` | Decode every row that has a layer default and resolve it through the preset and layer-row paths | Both paths report the row's kind id for every layer-capable block | `pictura-render::adjustment_kind_map_is_one_mapping`, `pictura-app::layer_row_kind_matches_the_store_kind` |
| `adjust_catalogue_control_kinds` | Call `list_adjustments` and compare the returned names with the catalogue's dialog rows | All sixteen dialog kinds are listed, including Color Lookup and Shadows/Highlights, and each has a non-empty page | `tst_control_editing::adjustmentsListEveryDialogKind` |
| `adjust_facts_gpu_parity_rows` | Run the GPU parity case over catalogue rows with a non-`None` route | Every routed row has a layer default, composites through both backends within 1 LSB, and `checked` equals the routed-row count | `pictura-render::adjustment_layers_match_cpu` |
| `adjust_facts_gpu_param_shape` | Plan Hue/Saturation with a colour range, then composite the same document through the CPU oracle | The GPU planner declines the layer, and the CPU result changes from the unadjusted baseline | `pictura-render::gpu_route_falls_back_for_unmodelled_parameters` |
| `adjust_facts_defaults_distinct` | Build every row's dialog and layer defaults with the same black/white context and compare their payloads | Brightness/Contrast, Hue/Saturation, and Photo Filter differ there; Gradient Map differs only with a non-default foreground/background pair | `pictura-render::layer_defaults_differ_deliberately` |
| Every supported kind creates a named layer | Request each of the sixteen adjustment kinds through the layer-creation path | A layer with that kind's block and the row title as its name is added | `pictura-app::adjustment_layer_covers_every_kind_and_rejects_unknown` |
| Unknown kind is refused | Request an unrecognised adjustment kind | No layer is added and the request fails | `pictura-app::adjustment_layer_covers_every_kind_and_rejects_unknown` |
| The new kinds open on neutral defaults | Compare layer and dialog payloads for Levels, Curves, Exposure, Vibrance, and Black & White, then open each layer editor | Those five layer payloads equal their dialog defaults, and each editor opens under the row title | `pictura-render::layer_defaults_differ_deliberately`, `pictura-render::adjustment_row_layer_defaults` |
| The panel creation menu offers every kind | Open the Layers panel New Fill / Adjustment menu in the Qt suite | Solid Color, Gradient, and Pattern plus all sixteen adjustment kinds give nineteen entries, and choosing an adjustment adds its layer | `tst_adjustments_panel::layersPanelAdjustmentMenuFollowsCatalogue` |
| `adjust_layer_color_lookup_preset` | Create a Color Lookup adjustment layer and resolve its stored preset name | The stored name resolves to index 0, the `None` preset | `pictura-app::color_lookup_layer_names_none_preset` |

## Verification record

Commands ran from the worktree root on 2026-11-24.

- `cargo fmt --all`: passed.
- `bash scripts/with-build-lock.sh -- bash -c 'cmake -S . -B build -G Ninja && cmake --build build --parallel'`: passed, 1,562 build steps. Two later rebuilds followed source fixes and passed, 275 steps each.
- `bash scripts/with-build-lock.sh -- cargo nextest run -p pictura-core -E 'test(/adjustment_kind/)' --status-level fail --final-status-level fail`: 2 passed.
- `bash scripts/with-build-lock.sh -- cargo nextest run -p pictura-codec -E 'test(/layer_content::defaults|adjustment_layers_round_trip_key_and_bytes/)' --status-level fail --final-status-level fail`: 4 passed.
- `bash scripts/with-build-lock.sh -- cargo nextest run -p pictura-render -E 'test(/adjustment_catalogue_covers_every_kind_once|adjustment_catalogue_layer_entries|adjustment_row_dialog_defaults|adjustment_row_layer_defaults|layer_defaults_differ_deliberately|adjustment_kind_map_is_one_mapping|preset_kind_mapping_covers_the_catalogue|decoded_blocks_match|saveable_and_menu_kinds_match_the_spec/)' --status-level fail --final-status-level fail`: 9 passed.
- `bash scripts/with-build-lock.sh -- cargo nextest run -p pictura_app -E 'test(/adjustment_layer_covers_every_kind_and_rejects_unknown|color_lookup_layer_names_none_preset|layer_row_kind_matches_the_store_kind|adjustment_catalogue_rows_resolve_commands|channel_mixer_adjustment_layer|same_kind_guard_rejects_different_or_undecodable_blocks/)' --status-level fail --final-status-level fail`: 6 passed.
- `bash scripts/with-build-lock.sh -- cargo nextest run -p pictura-render --features gpu -E 'test(/^gpu::route_tests::catalogue_rows_keep_every_fact_in_sync$|^gpu::eligibility_tests::gpu_route_falls_back_for_unmodelled_parameters$|^gpu::eligibility_tests::adjustment_without_a_gpu_kind_declines_at_any_rect$/)' --status-level fail --final-status-level fail`: 3 passed.
- `bash scripts/with-build-lock.sh -- cargo test -p pictura-render --features gpu --test gpu_parity adjustment_layers_match_cpu -- --ignored --nocapture`: 1 passed on the NVIDIA GeForce RTX 3090. All 9 routed kinds matched CPU within 1 LSB.
- `bash scripts/with-build-lock.sh -- env QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_(adjustments_panel|command_tree|image_adjustments|control_editing)$' --output-on-failure`: 4/4 CTest entries passed.
- `bash scripts/with-build-lock.sh -- cargo clippy --workspace --all-targets -- -D warnings`: passed after removing function-pointer equality derives and two needless test borrows.
- `openspec validate adjustment-facts-single-source --strict`: passed.
- `python3 scripts/check-test-plan.py`: passed, 1 change checked.
- `bash scripts/with-build-lock.sh -- bash scripts/check-file-size.sh`: passed, 5 over-budget files all allowlisted.

No focused acceptance check was skipped.

## Final aggregate and baseline proof

- `bash scripts/with-build-lock.sh -- env LC_ALL=C bash scripts/verify-fast.sh`: `fmt=ok`, `clippy_errors=0`, full nextest 6,802 passed and 136 skipped. Qt reported 4,428 passed, 14 skipped and 1 failed. The unified total was 11,733 passed, 150 skipped and 1 failed.
- The only aggregate failure was `PresentCacheTest::frameBoundedPaintMatchesCpu(gpu)`: `'samePixels(committed, cpu.image())' returned FALSE`.
- A detached clean worktree at `origin/master` commit `4e09c82f327b3e3917c0a2ed18e10393c61575aa` was built with `bash scripts/with-build-lock.sh -- bash -c 'cmake -S . -B build -G Ninja && cmake --build build --target tst_present_cache --parallel'`.
- `bash scripts/with-build-lock.sh -- env QT_QPA_PLATFORM=offscreen ./build/crates/pictura-app/cpp/tests/tst_present_cache 'frameBoundedPaintMatchesCpu:gpu'` failed with the same assertion at `tst_present_cache.cpp:1013` on clean `origin/master`.
- The post-test `verify-fast.sh` checks ran separately. File size, scoped guard, Flatpak, worktree hook, build caching, build lock, self-test budget, archive, icon provenance, shell seams, test plan, parity, source fragments, CPU budget, agent budget, agent build, heavy policy, agent guard and OpenSpec all passed. MSVC-string and colour-literal self-tests passed separately.
- `python3 scripts/check-milestone-names.py --self-test` failed with `AssertionError: a scoped scan missed a changed violation` on both this branch and clean `origin/master`. The actual milestone-name guard passed. This is a second baseline failure, not a branch regression.
