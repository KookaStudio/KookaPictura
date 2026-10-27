# Tasks

## 1. Tests first

- [x] 1.1 Add the catalogue contract tests in `crates/pictura-filters/src/catalogue/tests.rs`: `catalogue_covers_every_kind`, `catalogue_rows_match_kind_order`, `catalogue_rows_round_trip_their_facts`, `catalogue_defaults_are_in_range`, `every_catalogue_row_builds_from_its_defaults`, and `hdr_toning_is_engine_only`; run them and confirm they fail before the catalogue exists
- [x] 1.2 Add the range tests `add_noise_amount_has_one_range`, `surface_blur_threshold_has_one_range`, and `out_of_range_slot_is_refused`; confirm the Add Noise and Surface Blur cases fail on the current ranges
- [x] 1.3 Add the app tests `resource_backed_rows_resolve_through_the_app_seam`, `filter_descriptors_mirror_the_catalogue`, and `hdr_toning_is_not_a_filter_command`; keep the existing `filter_map_tests` names
- [x] 1.4 Add `accelerated_parity_fixture_matches_the_catalogue` and `gpu_plan_matches_the_row_policy` in `crates/pictura-render`; confirm the fixture check covers every accelerated kind and no other
- [x] 1.5 Rewrite `pictura-script::every_wired_filter_has_a_definition` to read `pictura_filters::catalogue::all()` and add the `pictura-filters` dev-dependency; confirm it fails until the catalogue exists

## 2. Catalogue core

- [x] 2.1 Add `pictura-filters/src/catalogue/mod.rs` with `FilterSpec`, `FilterParam`, `FilterControl`, `FilterDialog`, `AlphaPolicy`, `ApronPolicy`, `GpuPolicy`, `Build`, and the range/defaults accessors; verify the new module compiles on its own
- [x] 2.2 Move the 115 menu rows and add the `HdrToning` engine-only row into `catalogue/{paint,blur,distort,render,stylize,video,engine_only}.rs`, preserving paths, labels, parameter order, and defaults; verify `catalogue_covers_every_kind` and `catalogue_rows_match_kind_order`
- [x] 2.3 Extend `filter_variants!` so each row names its spec const and `FilterKind::ALL`, `name`, `engine`, `spec`, `moves_alpha`, `preview_apron`, and `cpu_outruns_gpu` derive from it; verify `catalogue_rows_round_trip_their_facts` and the existing `filter_reports_its_own_facts`
- [x] 2.4 Add `defaults()`, `arity()`, `range_for_slot()`, `default_filter()`, and `NullResources`; verify `catalogue_defaults_are_in_range` and `every_catalogue_row_builds_from_its_defaults`

## 3. Builders and the app seam

- [x] 3.1 Move the fixed-shape `mapping.rs` decoders and the `filter_map_choices.rs` arrays into `catalogue/build/`, split by family under the file-size cap; verify `parameter_values_override_defaults` and `every_kind_defaults_from_empty_and_rejects_wrong_arity`
- [x] 3.2 Move `gallery.rs` into `catalogue/build/gallery.rs`, keeping the pin, path, spin, and effects grammars; verify the six variable-list app tests named in `test-plan.md`
- [x] 3.3 Add `FilterResources` and the `AppResources` implementation over the Lens Blur, Displace, Liquify, AWA, and Vanishing Point stores; delete the app descriptor/mapping/gallery modules and keep `filter_from_kind_params` plus `commit_params`/`scale_pixel_params`; verify the five resource tests and `filter_descriptors_mirror_the_catalogue`
- [x] 3.4 Update `filter_map_tests.rs`, `helpers.rs`, `filter_tools.rs`, and `canvas_proxy.rs` to the catalogue paths and the `FilterKind::ALL.len()` count; verify `pictura-app`'s filter-map and filter-tools test modules pass
- [x] 3.5 Split any catalogue row or builder file that reaches the 800-line target into family modules and confirm `scripts/check-file-size.sh` needs no allowlist entry

## 4. Range single source

- [x] 4.1 Replace the ported dispatcher's scalar range literals with `check_slot(filter, slot, value)` and delete the Smart Blur mapping clamps; verify the existing ported, scenario, and property suites still pass
- [x] 4.2 Set Add Noise to `0.1..400` and Surface Blur threshold to `1..255`, keeping Wave, Shear, and texture option checks explicit; verify `add_noise_amount_has_one_range`, `surface_blur_threshold_has_one_range`, and `out_of_range_slot_is_refused`
- [x] 4.3 Check every catalogue default against its row range and every mapping clamp still present; verify `catalogue_defaults_are_in_range` and `catalogue_rows_round_trip_their_facts`

## 5. GPU eligibility

- [x] 5.1 Add the `GpuPolicy` values to the rows and gate `gpu_filter::plan` on them while keeping the parameter-dependent kernel match; verify `gpu_plan_matches_the_row_policy`
- [x] 5.2 Cross-check the GPU parity fixture against the catalogue's accelerated and CPU-preferred rows; verify `accelerated_parity_fixture_matches_the_catalogue` and the existing ignored `accelerated_filters_match_cpu_within_one_lsb`

## 6. Menu, gallery, and engine-only row

- [x] 6.1 Add `gallery_rank` to `FilterDescriptorRow`, the C++ `FilterCommandSpec`, and the accepted ranks; verify `filter_descriptors_mirror_the_catalogue` and `tst_filter_menu::rowsComeFromTheRustDescriptor`
- [x] 6.2 Delete `kGallery` and build `FilterGalleryDialog::categories()` from the bridge rows grouped by family and sorted by rank; verify `tst_filter_gallery::categoriesFollowCs6` and the remaining gallery suites
- [x] 6.3 Make `filter_kind_supported` and the app's filter-command entry point require a non-empty command path so the HDR Toning row stays engine-only; leave `filterCommandForKind` resolving the no-menu row. Verify `hdr_toning_is_engine_only` and `hdr_toning_is_not_a_filter_command`

## 7. Script catalogue

- [x] 7.1 Update `pictura-script/tests/catalogue.rs` to the catalogue and remove the descriptor source scan; verify `every_wired_filter_has_a_definition`
- [x] 7.2 Confirm the command table, golden rows, frozen ids, and `command_ids.h` are unchanged; verify `golden_catalogue_matches_the_captured_table`, `golden_const_names_match_the_captured_ids`, `cr_id_drift`, and `default_catalogue_shape_is_stable`

## 8. Integration verification

- [x] 8.1 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run -p pictura-filters -p pictura_app -p pictura-render -p pictura-script`, and the filter Qt suites through `ctest --test-dir build -R '^tst_(filter_menu|filter_gallery)'`
- [x] 8.2 Run `openspec validate --all --strict`, `scripts/check-test-plan.py`, and `scripts/check-file-size.sh`; confirm the change has no non-archived sibling and no allowlist increase
- [x] 8.3 Run the negative compile check for `fc_catalogue_missing_row` in a scratch worktree, record it in the PR body, and list the manual row as a reviewer checklist item
