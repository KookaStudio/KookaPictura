# Tasks

## 1. Plan scaffold and test hooks

- [x] 1.1 Add `crates/pictura-render/src/composite_plan.rs` with `CompositePlan`, `PlanNode`, `Body`, and the builder; build one plan in `composite_whole` and execute it; verify `banded_compositing_equals_one_canvas` still passes.
- [x] 1.2 Make `BlendCtx`, `MaskGate`, `RasterGate`, and `VectorGate` cloneable and derive `Clone, Copy` for `pictura_core::VectorCoverage`; verify `ctx_masks_match_legacy` passes.
- [x] 1.3 Add `cfg(test)` counters for adjustment decodes (keyed by the block's `Plane::ident`), plan-node resolutions, mask-gate resolves, per-key source lookups/fetches (like `pass_build_count`), and native-plane resolves (in `SourcePlanes::resolve`); verify `cargo nextest run -p pictura-render` still passes.

## 2. Write the new tests before the migration

Each test is written against the counters or behavior named below and must fail for the stated reason until its implementation task lands.

- [x] 2.1 Write `banded_composite_resolves_each_layer_once`; verify it fails because a banded composite resolves per band.
- [x] 2.2 Write `adjustment_decodes_once_per_composite`; verify it fails because each band decodes the adjustment.
- [x] 2.3 Write `sources_resolve_once_per_composite`; verify it fails because each band fetches the source and pass.
- [x] 2.4 Write `hidden_layer_resolves_nothing` with a visible layer as a positive control; verify it fails until the plan exists.
- [x] 2.5 Write `banded_reuses_resolved_mask_and_native_state`; verify it fails because each band resolves the mask gates and native planes.
- [x] 2.6 Write `banded_composite_does_not_materialize_a_fractured_plane`; verify it fails because `content_key` hashes plane bytes.
- [x] 2.7 Write `content_key_invalidates_on_edit`; this is a regression guard that may pass before the change; require both the new key and byte equality with a purge-then-composite for a channel, mask, and adjustment edit.
- [x] 2.8 Write `clipping_group_reuses_base_content`; verify it fails because the `opaque` and `coverage` variants re-resolve the base (the proxy variant does not).

## 3. Migrate the composite routes onto the plan

- [x] 3.1 Move the pixel, isolated/pass-through group, adjustment, and fill routes onto `PlanNode`; derive destructiveness from the decoded `Body`/`ContentRoute::Fill` instead of `is_destructive_adjustment`; verify `ctx_route_callers_match_legacy` passes.
- [x] 3.2 Move the smart-source, smart-filter, and type-source routes onto the plan's resolved sources; verify `banded_equals_whole_with_object_layers` and `sources_resolve_once_per_composite` pass.
- [x] 3.3 Build one plan in `composite_banded`, `composite_rgba_region`, `composite_native`, `render_session`, and `gpu/clip`; render_session builds one full-document plan, executes `&roots[k..]`, and passes `roots[0]` as the knockout base; gpu/clip uses `CompositePlan::build_for_layers`; verify `banded_compositing_equals_one_canvas`, `a_region_composite_matches_the_full_one`, the native-depth tests, `banded_composite_resolves_each_layer_once`, `hidden_layer_resolves_nothing`, and `banded_reuses_resolved_mask_and_native_state` pass.
- [x] 3.4 Refactor the `is_destructive_adjustment`/`no_passes` call sites so pass building takes the decoded adjustment and does not call `decode_adjustment` again; verify `adjustment_decodes_once_per_composite` passes.
- [x] 3.5 Make `pattern_driven` skip invisible layers so `banded()` does not decode their effects; verify `hidden_layer_resolves_nothing` passes.

## 4. Synthetic composite variants

- [x] 4.1 Add `BlendOverride` (including `isolated_group`) and replace the clipping `opaque`/`full`/`proxy` clones, the coverage clone, and the adjustment `gated` clone with a plan node plus overrides; apply the override in the legacy per-pixel body and run a clipping case under the legacy switch; verify `a_clipped_layer_shows_only_over_its_base`, `ctx_route_callers_match_legacy`, and `clipping_group_reuses_base_content` pass.

## 5. Identity-based keys

- [x] 5.1 Hash `Plane::ident()` instead of plane bytes in `content_key`, `effect_digest`, and `chain_hash`; verify `text_state_invalidates_sources_and_passes`, `effect_passes_invalidate_on_style_or_rect`, `editing_text_invalidates_the_composite`, `replacing_smart_object_contents_invalidates_the_composite`, `banded_composite_does_not_materialize_a_fractured_plane`, and `content_key_invalidates_on_edit` pass.

## 6. Profile and integration

- [x] 6.1 Extend `crates/pictura-render/tests/composite_profile.rs` with a 4000² stack carrying Curves, Color Lookup, a proxy-less type layer, and a styled layer; verify `cargo test -p pictura-render --test composite_profile -- --ignored --nocapture` prints a timing for it.
- [x] 6.2 Record the before (`origin/master`) and after timings in the PR body; verify the table lists both runs on the same machine and build profile (manual: wall time is machine-dependent and no fixed threshold is set).
- [ ] 6.3 Run `bash scripts/verify-fast.sh` and the oracle suites; verify it passes with no baseline changes.
