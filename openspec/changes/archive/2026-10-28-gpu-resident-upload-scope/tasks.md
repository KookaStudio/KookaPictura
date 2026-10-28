# Tasks

## 1. Resident store and gauges

- [x] 1.1 Write `gpu::resident::tests::get_returns_a_kept_upload`, `get_leaves_the_store_unchanged`, `put_evicts_past_the_byte_budget`, `put_refuses_an_upload_over_the_budget`, `put_keeps_the_entry_count_bounded`, `put_replacing_a_key_keeps_one_entry`, and `put_reports_post_eviction_stats` against a `Resident<u64>` stub; run `cargo nextest run -p pictura-render resident` and confirm they fail to compile until 1.2
- [x] 1.2 Replace the `Vec`-scanning `Resident` with a `HashMap<UploadKey, (V, u64)>` keyed store, an insertion-order `VecDeque<UploadKey>`, a running byte total, `MAX_ENTRIES = 1024`, a manual `impl<V> Default`, and a `Hash` derive on `UploadKey`; make `get` take `&self`, refuse an upload larger than `BUDGET`, and make `put` subtract and re-queue on replace, evict from the front, and return the post-eviction `(entries, bytes)`; verify 1.1 passes
- [x] 1.3 Add `resident_entries` and `resident_bytes` atomics to `pictura-render/src/metrics.rs`, publish the `(entries, bytes)` `Resident::put` returns from `kept_or_upload`, add both fields to `Counters`, and print them in the app metrics JSON; verify `cargo nextest run -p pictura-render metrics` and the existing `tst_control_metrics` suite are green

## 2. Document-scoped source uploads

- [x] 2.1 Update `gpu::resident::tests::a_source_key_follows_its_planes_and_geometry` for the region-free key (document size in the key; no region argument; document size and grayscale change the key) and keep `an_unstamped_plane_is_never_cached`; verify `cargo nextest run -p pictura-render resident` fails until 2.2 and passes after
- [x] 2.2 Change `source_key` to key on the plane stamps and lengths, the raw layer rect, the document width/height, and the grayscale flag; remove the region from `UploadKey::Source`; verify 2.1 passes
- [x] 2.3 In `Gpu::build_source`, compute the region and document source geometry, look up the document-scoped key, assemble only the region geometry on a miss, and return the document layout on a hit; add the `keep` flag to `kept_or_upload` so a sub-region upload is not retained; update the module and function docs that still describe region-keyed uploads; verify `cargo nextest run -p pictura-render region` (the CPU region tests) and `cargo build -p pictura-render` succeed, leaving the GPU hit path to group 4
- [x] 2.4 Keep the stale-upload contract: verify `cargo nextest run -p pictura-render --features software-vulkan --profile gpu --run-ignored all resident_uploads_are_reused_and_never_stale` passes under the new key (the gpu-parity CI job runs it on lavapipe)

## 3. Mask retention

- [x] 3.1 Update `gpu::resident::tests::a_mask_key_needs_a_stamped_plane_only_when_it_samples_one` to expect no key for constant coverage (no mask, and a disabled mask that carries stamped data) and to keep the sampled-mask key checks; verify it fails until 3.2 and passes after
- [x] 3.2 Make `mask_key` return `None` for a layer with no enabled data-carrying mask, drop the `Option` from `UploadKey::Mask.sampled`, and pass `doc` into `build_mask` so coverage is retained only when the region is the whole document; update all four call sites (`composite_layer`'s three and the test-only `run_profile` at `gpu/mod.rs:1101`); verify 3.1 and `cargo nextest run -p pictura-render region` pass

## 4. Device and region-hit evidence

- [x] 4.1 Stamp every channel plane in `gpu_parity::region_scenes` (and the masked layer's mask plane) so `region_matches_full_slice` and `full_region_equals_full_composite` exercise retained whole-document uploads; assert `resident_hits` grows inside `assert_region_slice`; verify with `cargo nextest run -p pictura-render --features software-vulkan --profile gpu --run-ignored all region_matches_full_slice full_region_equals_full_composite`
- [x] 4.2 Write `resident_uploads_stay_bounded_across_region_presents` in `crates/pictura-render/tests/gpu_parity/region.rs`: stamp four full-canvas layers (one with a stamped sampled mask), seed with a whole-document composite, then run 100 region composites over moving rectangles and assert every backend is `Backend::Gpu`, `resident_entries` and `resident_bytes` do not grow, `resident_hits` grows by four per present, and the sampled-mask misses grow by one per present; then composite a second document with fresh stamps over a sub-rect (a keyed miss that must not retain anything) and compare its following whole-document composite to `composite_rgba` within ±1 LSB; finally assert the seeded document's whole-document composite adds zero misses and five hits (four sources plus the sampled mask), and that the gauges are non-zero
- [x] 4.3 Run the new test on a Vulkan device or in the lavapipe shape: `cargo nextest run -p pictura-render --features software-vulkan --profile gpu --run-ignored all resident_uploads_stay_bounded_across_region_presents`

## 5. Integration checks

- [x] 5.1 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, and `cargo test --workspace --doc`
- [x] 5.2 Run `bash scripts/verify-fast.sh`, `openspec validate --all --strict`, and `openspec archive gpu-resident-upload-scope`; verify only `openspec/changes/archive/` remains for this change
