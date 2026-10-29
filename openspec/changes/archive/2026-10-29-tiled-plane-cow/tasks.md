# Tasks

## 1. Plane band storage (tests first)

- [x] 1.1 Write `fracturing_a_plane_copies_no_pixels`, `a_band_write_forks_only_its_band`, `contiguous_and_band_reads_agree`, `fracture_preserves_every_byte`, `a_whole_plane_write_flattens`, and `degenerate_planes_fracture_safely` in `crates/pictura-core/src/plane.rs`; expected red until 1.2–1.8; verify with `cargo nextest run -p pictura-core plane`
- [x] 1.2 Add the band model to `Plane<T>`: one `base: Arc<[T]>`, `bands: Vec<Band<T>>` with `Shared(Range<usize>)` and `Owned(Arc<[T]>)`, a recorded width, `fracture(width)`, `bands()`, `band(i)`, `row(y)`, `row_mut(y)`, and `write_band(i)` that writes in place only for an `Owned` band with `strong_count == 1` or a `Shared` band whose base has `strong_count == 1`, and otherwise forks that band; verify with `cargo nextest run -p pictura-core plane`
- [x] 1.3 Add the contiguous-read bridge and the flattening rule: `as_slice()` returns the base while no band has diverged and otherwise one cached contiguous view; `as_mut_slice`/`DerefMut` materialize one contiguous allocation, replace the band view, and reset the cache; verify with `cargo nextest run -p pictura-core plane`
- [x] 1.4 Add inherent `len()` and `is_empty()` on `Plane` that read the logical length without materializing; verify with `cargo nextest run -p pictura-core plane`
- [x] 1.5 Keep `Debug`, `PartialEq`, `AsRef`, `IntoIterator`, `FromIterator`, and `From<Vec>` correct on banded planes and cover them with `banded_planes_keep_their_std_traits`; verify with `cargo nextest run -p pictura-core plane`
- [x] 1.6 Make the materialization counter `pub` and always compiled (not `#[cfg(test)]`), and assert it counts a flat write and a materializing read with `materialization_counter_is_public_and_counts`; verify with `cargo nextest run -p pictura-core plane`
- [x] 1.7 Add a test-only band-allocation accessor (`shares_band` / band pointer) so tests can assert per-band sharing, and cover it with `band_shares_reports_every_band`; verify with `cargo nextest run -p pictura-core plane`
- [x] 1.8 Keep `build`, `extend_from_slice`, `extend`, `clear`, `truncate`, `set_stamp`, `ident`, and `shares` correct on both variants, with plane-level stamp and ident cleared by every write and `shares` true only when every band is the same allocation; update the existing plane tests; verify with `cargo nextest run -p pictura-core`

## 2. History band sharing (tests first)

- [x] 2.1 Write `opening_a_document_shares_bands_with_history`, `a_capture_scans_only_changed_bands`, `a_live_write_forks_only_touched_bands`, and `a_written_band_is_not_shared_with_history` under `crates/pictura-app/src/history/tests.rs` (or a module it declares); expected red until 2.2–2.5; verify with `cargo nextest run -p pictura_app history`
- [x] 2.2 Fracture each tracked live plane before sharing in the open path and remove `private_copy` and its import; still assign fresh stamps to both the live document and the history cursor state; verify with `cargo nextest run -p pictura_app history`
- [x] 2.3 Make `diff_into` skip bands whose allocation matches and tile-diff only changed bands, make the tile apply write inside the band it targets, and use the inherent `len`/`is_empty` in `each_plane`/`each_plane_mut` so the scan does not materialize; verify with `cargo nextest run -p pictura_app history`
- [x] 2.4 Make `shared_copy` and `stamp_agreed` band-aware so a plane whose bands are shared is not copied and only unshared bands or planes are copied; verify with `cargo nextest run -p pictura_app history`
- [x] 2.5 Make `retained_plane_bytes` count each band allocation once and the base allocation once without materializing; verify with `cargo nextest run -p pictura_app history`
- [x] 2.6 Update the `shares_no_plane` assertions and the four named existing tests (`structural_captures_retain_only_new_planes`, `undoing_a_structural_edit_keeps_the_live_planes`, `structural_capture_with_a_duplicated_layer_round_trips`, `a_capture_after_a_duplicate_stays_a_delta`) to band semantics; verify with `cargo nextest run -p pictura_app structural` and `cargo nextest run -p pictura_app live_restores_match_a_full_snapshot_history`
- [x] 2.7 Replace the old "stroke commit materializes no plane" check with `a_stroke_path_materializes_no_plane_and_scans_only_touched_bands`, asserting the stroke write path materializes no plane while the capture scan visits only the touched bands; verify with `cargo nextest run -p pictura_app history` and `cargo nextest run -p pictura-paint stroke`
- [x] 2.8 Keep `a_capture_after_a_duplicate_stays_a_delta` green as a band-aware delta after a duplicate; verify with `cargo nextest run -p pictura_app a_capture_after_a_duplicate_stays_a_delta`

## 3. Hot readers, writer migration, and profile (tests first)

- [x] 3.1 Write `a_stroke_forks_only_touched_bands` in `crates/pictura-paint/src/stroke/tests.rs`; expected red until 3.2; verify with `cargo nextest run -p pictura-paint stroke`
- [x] 3.2 Migrate `plane_slices` and the dab loops in `crates/pictura-paint/src/stroke/raster.rs` and the region apply in `crates/pictura-paint/src/stroke.rs` (the `dst[k] = Some(c.data.as_mut_slice())` path) to the band writer; `fill.rs` and filters stay whole-plane and flatten; verify with `cargo nextest run -p pictura-paint`
- [ ] 3.3 Split `crates/pictura-paint/src/stroke.rs` (1177 LOC against the 1200 cap) along the band-writer/region-apply seam so it keeps headroom; verify with `bash scripts/check-file-size.sh` and `cargo nextest run -p pictura-paint`
- [x] 3.4 Add `composite::channel_plane` and migrate the hot whole-plane readers to `Plane::row`: `composite_rows::composite_pixels`, `gpu/assemble.rs` (row source and per-pixel fallback), the route/existence checks (`composite_route`, `smart_source`, `text_render`), `patch_buffer_region`, `crop_planar`, and the stroke's `Saved::save`; verify with `cargo nextest run -p pictura-render` and the stroke profile reporting zero materializations
- [ ] 3.8 Migrate the remaining cold whole-plane readers (filter.rs, effect mattes, matting) to rows so a filter or effect on a banded plane does not materialize it; verify with the filter and effect suites
- [ ] 3.5 Split `crates/pictura-render/src/composite.rs` (959 LOC against the 1200 cap) along the channel-view seam so it keeps headroom; verify with `bash scripts/check-file-size.sh` and `cargo nextest run -p pictura-render`
- [x] 3.6 Use the inherent `len()` in `crates/pictura-render/src/gpu/resident.rs` so the resident key does not materialize a banded plane; verify with `cargo nextest run -p pictura-render resident`
- [x] 3.7 Add ignored `large_document_open_profile` (8 layers, 4000×4000 RGBA) printing the unique tracked-plane bytes across the live document and all retained history after open and after one stroke capture, materializations during the stroke, and first-dab time; verify with `cargo test -p pictura_app large_document_open_profile -- --ignored --nocapture`

## 4. Integration and archive

- [ ] 4.1 Run the targeted checks: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run -p pictura-core`, `cargo nextest run -p pictura_app`, `cargo nextest run -p pictura-paint`, `cargo nextest run -p pictura-render`, `cargo test --workspace --doc`, and `openspec validate tiled-plane-cow --strict`; verify all pass
- [ ] 4.2 Archive with `openspec archive tiled-plane-cow` so only `archive/` remains and the deltas merge into the main specs; verify with `openspec validate --all --strict`
- [ ] 4.3 Run `bash scripts/verify-fast.sh` after archiving (it runs `check-test-plan.py` and `check-openspec-archived.sh`, which fails while an unarchived change exists); verify it passes
