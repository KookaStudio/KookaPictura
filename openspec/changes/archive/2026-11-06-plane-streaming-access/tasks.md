# Tasks

## 1. Tests

- [x] 1.1 Add the `pictura-core` `plane/access/tests.rs` cases for rows,
  `read_rect_into`, `read_into`, bands, the mutable forms, and the guard.
- [x] 1.2 Add `pictura-render` `tests/plane_streaming.rs`: composite, trim and
  untrim, selection move, and Free Transform on diverged banded planes, each
  under `assert_no_materialization` and compared against flat planes.
- [x] 1.3 Make the heap guards in `tests_profiles/memory.rs` report
  materializations and assert 0 for the `STREAMED` scenarios.

## 2. Core

- [x] 2.1 Add `rows`, `rows_mut`, `read_rect_into`, `write_rect_from`,
  `read_into`, `for_each_band`, `for_each_band_mut` and
  `assert_no_materialization` in `plane/access.rs`.
- [x] 2.2 Move the `layer_bounds.rs` grow and shrink to row access.

## 3. Render

- [x] 3.1 Compositing: mask gate and `mask_value` → `sample`; feathered mask →
  `row`; content probe → `channel_plane`; effect alpha matte → `row`;
  `rgba_frame` and `with_merged_alpha` → `read_into`.
- [x] 3.2 Move: `lifted_copy`, `composite_over`, `grow_to`, `trim_layer`,
  `layer_is_inert`, and clipboard copy and clear.
- [x] 3.3 Free Transform: `resample_plane` reads rows from the plane.
- [x] 3.4 Layer via Copy/Cut (`via.rs`): `sample` and `row_mut`.

## 3a. App

- [x] 3a.1 `history/planes.rs` `copy_plane`: a parallel band-by-band copy, so a
  snapshot (M8) does not materialize.

## 4. Verification

- [x] 4.1 `cargo fmt`, `cargo clippy --workspace --all-targets -D warnings`,
  `cargo nextest run -p pictura-core -p pictura-render`, and
  `cargo test -p pictura-render --test document_oracle`, with no golden
  changed.
- [x] 4.2 `cargo nextest run -p pictura_app heap_peak`: all 12 pass, and
  M2–M10 report 0 materializations.
- [x] 4.3 `scripts/check-test-plan.py`, `openspec validate --all --strict`,
  `scripts/guard.sh`, and the file-size check.
- [x] 4.4 Archive the change.
