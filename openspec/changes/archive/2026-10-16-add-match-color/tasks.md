# Tasks: add-match-color

## 1. Engine (`pictura-adjust`)

- [x] 1.1 `src/match_color.rs`: Lab conversion helpers, `MatchColorStats`,
      `match_color_stats` (alpha- and selection-weighted), `MatchColorParams`
      validation, the `match_color` kernel (transfer + neutralize + luminance
      + colour intensity + fade + flat-target guard + self-match identity).
- [x] 1.2 `types.rs` `Adjustment::MatchColor`, `apply.rs` dispatch, `lib.rs`
      exports; `pictura-render` re-exports the types.
- [x] 1.3 `tests/match_color.rs`: identity (self-match and fade 100), alpha
      preserved, validation errors, mean shifts toward the source, neutralize
      removes the cast, colour intensity 1 is exactly gray, luminance
      monotonic, fade half-way blend, flat-target guard, selection-weighted
      statistics; native-depth refusal asserted.

## 2. App bridge (`pictura-app`)

- [x] 2.1 `impl_filters.rs`: `ignore_selection` parameter on
      `apply_op_active_region` (existing call sites pass `false`).
- [x] 2.2 `src/cxxqt_object/image_match_color.rs`: target/source statistics,
      preview, apply; registered in `build.rs` and `cxxqt_object.rs`.

## 3. Dialog and menu (`pictura-app` C++)

- [x] 3.1 `match_color_dialog.{h,cpp}`: Source/Layer combos, the three
      selection checkboxes, Neutralize, Luminance / Colour Intensity / Fade
      sliders, Preview, Save/Load Statistics; added to `CMakeLists.txt`.
- [x] 3.2 `frame_menus_adjust.cpp`: replace the ponytail stub with the
      Match Color wiring (RGB-only enablement, ellipsis label, modal dialog).

## 4. Tests

- [x] 4.1 `tst_match_color.cpp` in `cpp/tests/` (+ tests CMake list): row
      enabled with the ellipsis, dialog opens, Cancel restores bit-identically
      with no state, OK commits one "Match Color" state, a second document
      appears as a source and changes the target, Ignore Selection applies
      outside the selection, Save/Load Statistics round-trips.
- [x] 4.2 `tst_image_adjustments.cpp`: Match Color moves from the
      disabled-stub assertion to the enabled set.

## 5. Verification

- [x] 5.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo nextest run --workspace`, `cargo test --workspace --doc`.
- [x] 5.2 CMake build; `ctest --test-dir build -R '^tst_' --output-on-failure`.
- [x] 5.3 `bash scripts/verify-fast.sh`; `openspec validate --all --strict`;
      `docs/dev/STATE.md` updated (docs commit carries `TASK-ALLOWS-DOCS`).
