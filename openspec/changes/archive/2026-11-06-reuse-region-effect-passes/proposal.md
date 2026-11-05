# Proposal

## Why

Issue [#1180](https://github.com/KookaStudio/KookaPictura/issues/1180) reports a warm 128-row styled-layer region taking 1.41 s while a 256-row banded region takes 0.056 s. Short regions unconditionally rebuild effect passes even when a full composite has already cached the same content's passes.

## What Changes

- Reuse ready, content-matching whole-layer effect passes for non-banded region composites through the existing clipped replay.
- On a missing or in-flight whole-layer pass, retain bounded transient region builds without populating the whole-layer cache or waiting for its build.
- Keep pattern-driven passes transient and preserve content-change invalidation for paint and mask edits.
- Add deterministic reuse, freshness, and region/full-slice checks plus an ignored bevel performance profile.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/layer-source-cache`: require ready whole-layer pass reuse for short regions, refine the existing region-build requirement to apply on misses and pattern-driven paths, and specify a bevel profiling comparison.

## Impact

- Implementation: `crates/pictura-render/src/layer_source/mod.rs`; existing CPU/GPU pass replay should need no change.
- Coverage: layer-source tests, layer-effect cache tests, and `crates/pictura-render/tests/source_cache_profile.rs`.
- No new dependencies, public API changes, pixel-baseline changes, or bevel algorithm changes. Parallel cold-pass preparation (#1179) and blur optimization (#1178) remain separate.
