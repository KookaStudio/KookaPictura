# Design

## Context

The base is freshly fetched `origin/master` at `b40039f7d`. The parent codebase graph is generation `2026-11-24T19:18:06Z` at `b9ff5feb`; six RAW test files changed after that snapshot, so their current source was read directly. The filters crate already uses `tests/<topic>/main.rs`, and render already uses `tests/gpu_parity/main.rs`.

The process-wide exceptions are visible in source. `hostile_inputs`, `streamed_save`, and `merged_preview_buffers` install allocators. `text_gamma_off` changes text gamma. `backend_pin` and `backend_cpu` depend on a fresh backend-selection process. `canvas_mask_domain` changes the memory host and clears refusal state. `cold_engine_profile` relaunches its own test binary. The GPU drivers already serialize the state they share.

## Goals / Non-Goals

**Goals:**

- Reduce integration test link targets without changing test behavior.
- Keep process-wide state isolated by process boundary.
- Make the eight profiling suites runnable as examples.
- Preserve exact test and ignore-reason inventories.
- Keep documented commands and moved-file references accurate.

**Non-Goals:**

- Removing `pictura-render` from codec dev-dependencies. That is #1565 and remains open.
- Changing runtime code, public APIs, dependencies, CI jobs, or nextest's GPU profile.
- Running builds, tests, lint, format checks, OpenSpec validation, or repository check scripts in this change.
- Archiving the change or committing the `docs/` updates.

## Decisions

### Codec driver map

Use four new drivers and retain `oracle`:

| Driver | Former suites |
|---|---|
| `camera_raw` | `dng_camera_colour`, `dng_demosaic`, `dng_linear`, `dng_reader`, `dng_writer_oracle`, `raw_develop`, `raw_rw2`, `raw_rw2_codings`, `raw_camera_matrices`, `raw_cr2`, `raw_orf`, `raw_arw`, `raw_cr3_colour`, `raw_rw2_colour`, `raw_cr3`, `raw_nef` |
| `metadata` | `action_file`, `color_settings_file`, `icc_embed`, `icc_oracle`, `iptc_write_oracle`, `metadata_oracle`, `preset_library`, `profile_assignment_oracle`, `raw_settings_xmp` |
| `psd` | `agpsd_oracle`, `channel_budget`, `channel_sense_oracle`, `color_mode_oracle`, `content_constructors`, `depth_oracle`, `empty_layer_oracle`, `guide_resource_oracle`, `layer_content`, `layer_record_flags`, `mask_link_state`, `mask_params_oracle`, `offcanvas_mask`, `path_resource_oracle`, `tiled_planes`, `vector_fill_oracle`, `vector_mask_oracle` |
| `save` | `pack_codec_benchmark`, `save_golden`, `save_oracle`, `web_oracle`, `webp_oracle` |

Move the existing `tests/dng/` and `tests/raw/` helper directories under `tests/camera_raw/`. The moved RAW suites keep their relative `#[path = "dng/..."]` and `#[path = "raw/..."]` attributes. PSD suites that use `include_bytes!("fixtures/...")` move one level deeper, so those paths gain `../`.

Keep `oracle.rs`, `hostile_inputs.rs`, `streamed_save.rs`, and `merged_preview_buffers.rs` as top-level binaries. This reduces codec from 51 integration binaries to eight.

### Render driver map

Use three new drivers:

| Driver | Former suites |
|---|---|
| `document` | `apply_image`, `calculations`, `cold_compare_exit`, `document_oracle`, `native_depth_roundtrip` |
| `oracle` | existing `oracle.rs` contents plus `adjustment_oracle`, `clipping_oracle`, `knockout_oracle`, `merge_oracle`, `svg_oracle` |
| `pdf` | `pdf_live_text`, `pdf_photoshop_open` |

Move `oracle.rs` to `oracle/main.rs`. Its binary name stays `oracle`, so the nextest GPU filter and existing `test(/fixture_scene/)` selection keep working.

Keep `backend_pin`, `backend_cpu`, `canvas_mask_domain`, `cold_engine_profile`, `gpu_filter_parity`, `gpu_parity`, and `text_gamma_off` as top-level binaries. The issue's base had 28 render integration binaries; the rebase base (`origin/master` after #1612) has 30, because that PR added `codec_content_roundtrip` and `codec_layer_oracles`, which now belong to the `document` and `oracle` drivers. After moving eight profiling suites out, render has 10 integration binaries (8 at the issue base plus the two new suites folded into existing drivers).

### Profiling examples preserve both entry points

Move these files unchanged in behavior:

- `bevel_cold_composite_profile`
- `composite_profile`
- `effect_cold_latency`
- `gpu_text_gamma_bench`
- `per_layer_bench`
- `resident_upload_profile`
- `selection_move_perf`
- `source_cache_profile`

Replace each `#[test]` with `#[cfg_attr(test, test)]` and each `#[ignore = "..."]` with `#[cfg_attr(test, ignore = "...")]`. The reason string stays exact. Add `#[cfg(not(test))] fn main()` to call the profiling functions. `source_cache_profile` runs all three profiles in source order.

This keeps `cargo test --example <name> -- --ignored --nocapture` working and makes `cargo run --example <name>` execute the same code. The two GPU-gated examples keep their crate-level feature gate, so their documented commands include `--features gpu`.

### Target discovery stays convention-only

Cargo already discovers `tests/<name>/main.rs` and every file in `examples/`, so no test target is declared. The one exception is `required-features`: `gpu_text_gamma_bench` and `resident_upload_profile` keep a crate-level `#![cfg(feature = "gpu")]`, and Cargo builds examples before evaluating that attribute, so without a `required-features = ["gpu"]` entry a default build fails with "`main` function not found". The two `[[example]]` entries carry only `name` and `required-features`. The stable names of retained render binaries mean `.config/nextest.toml` and CI target selectors need no change.

### The consolidated RAW driver allows duplicate helper modules

Each camera RAW suite declares the shared synthetic builders with a relative `#[path]` attribute (`dng/builder.rs`, `dng/ljpeg_encode.rs`, `raw/*_builder.rs`). One binary now loads those files through several `mod` declarations, which `clippy::duplicate_mod` rejects at `-D warnings`. The driver root sets `#![allow(clippy::duplicate_mod)]`; each helper is still a private module of the moved suite, so behavior is unchanged and the alternative (rewriting every `#[path]` into shared `use` paths) is a larger, riskier diff.

### Reference updates are mechanical

Update current READMEs, active specs, agent guidance, and the Serena command memory to the new paths and driver names. Update archived OpenSpec run strings so they remain runnable. Historical prose that only records where a test was added is updated when it names a moved file. The only `docs/` edits are run-string or moved-file reference updates, and they are flagged for a later `TASK-ALLOWS-DOCS` commit.

## Alternatives considered

- **Leave files in place and include them from drivers.** Cargo would still build each top-level file as its own integration binary, so the link count would not fall.
- **Put global-state suites behind locks in shared drivers.** Allocators cannot be scoped to a module, backend and gamma changes can race, and `cold_engine_profile` depends on its executable name. Process isolation is simpler and matches the existing contracts.
- **Delete `#[test]` and `#[ignore]` when making examples.** This would lose the test inventory and exact ignore reasons. Conditional attributes preserve both entry points.
- **Add a test-inventory script or dependency.** The change is a pure move. Source comparison and existing tests provide the check without another repository tool.

## Risks / Trade-offs

- **Module qualification changes nextest names.** Update documented filters and review the source inventory. Existing broad package filters keep working.
- **`source_cache_profile` runs all three profiles.** The former exact-test command becomes a full example run; `cargo test --example source_cache_profile <name>` still selects one test when needed.
- **`clippy --all-targets` still checks examples.** It no longer builds eight test harnesses with zero default tests, while example compilation remains covered. The consolidated camera RAW driver needs `#![allow(clippy::duplicate_mod)]` because its suites share synthetic-builder files by `#[path]`.
- **The graph predates current RAW tests.** Direct source reads cover the changed files; the final review checks the actual worktree diff rather than trusting graph freshness.
- **Two all-ignored render suites arrived after the issue base.** #1612 added `codec_content_roundtrip` and `codec_layer_oracles` as integration binaries; dropping either would lose tests, so each moved into an existing driver instead of being ignored.
- **#1565 is still open.** Codec tests keep their current render dev-dependency. The driver reduction composes with #1565 but does not depend on its unmerged edits.

## Migration plan

Move source files, add driver declarations, add example entry points, then update references. No data migration or runtime rollout is needed. Rollback is a revert of the test-layout change. Build and verification commands are intentionally deferred to a later checked phase.
