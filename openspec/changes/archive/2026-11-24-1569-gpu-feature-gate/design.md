# Design

## Context

See `proposal.md` — Why. The wgpu stack reaches the crate through four doors:

1. `crates/pictura-render/Cargo.toml` — `wgpu` and `pollster` are plain
   dependencies (and `wgpu` again for `cfg(windows)` with the `dx12` feature).
2. `src/lib.rs` — `pub mod gpu;`, a 25-name `pub use gpu::{...}`, plus
   `pub mod gpu_filter`'s re-export of `apply_filter_active` and
   `filter_gpu_available`.
3. Three non-GPU modules that name GPU internals: `composite_request.rs`
   (`gpu::gpu_available`, `gpu::composite_gpu_region`), `filter.rs`
   (`crate::gpu_filter::apply_filter_active`) and `metrics.rs`
   (`crate::GpuError` and its decline counters). A fourth, `document_composite.rs`,
   names only the `Backend` type.
4. The `wgpu`-typed `Backend` enum, which those modules *and* the CPU document
   cache use to report which backend produced a buffer.

Only (4) is shared with strictly-CPU code, so `Backend` is the seam. Everything
that is genuinely GPU (the compositor, the stroke kernel, the filter kernels,
backend selection, the probe) is separable.

Measured before the change: `cargo tree -p pictura-render -e normal` names 138
distinct `name version` packages; `Cargo.lock`'s `wgpu` closure is 103 package
entries (101 names) of 250. A cold `cargo check -p pictura-render` in an empty
target directory is 17.0 s wall.

## Goals / Non-Goals

**Goals:**

- A default-feature build of `pictura-render` compiles and tests without `wgpu`.
- Enabling `gpu` restores the exact public surface the app already calls, with
  the same names and shapes.
- `pictura-app` builds and behaves as today; the `gpu-parity` job keeps its
  `--features software-gpu` spelling.
- No GPU test is deleted or weakened.

**Non-Goals:**

- No change to the GPU kernels, the probe, the backend pin, the CPU compositor,
  or any pixel result.
- No new crate, no new dependency, no public API addition beyond the feature.
- No `Cargo.lock` shrink effort: an optional dependency stays in the lock.
- No change to what `pictura-app` links; the application still ships GPU support.

## Decisions

### D1 — Feature name `gpu`, default off

`[features] gpu = ["dep:wgpu", "dep:pollster"]`, and `software-gpu = ["gpu"]`
keeps the CI spelling and makes it imply the feature. Both `wgpu` entries (the
Linux one and the `cfg(windows)` one with `dx12`) become `optional = true`; a
`dep:wgpu` reference enables every target-specific declaration of that name.

Default features stay empty. This is the whole point: a CPU-only consumer
(`pictura-codec`'s dev-dependency, a render test binary built by hand) sees no
wgpu. The application opts in, so the normal end-user build is unchanged.

Alternative considered: a `default = ["gpu"]` with an opt-out. Rejected — every
existing CPU-only invocation would keep paying the closure until someone edits
it, and the win would be conditional on new flags.

### D2 — `Backend` moves to an unconditional `src/backend.rs`

`Backend` is a two-variant enum with no wgpu in it, but the *reporting* API that
carries it is CPU code: `CompositeRequest`, `composite`, `composite_with_blend_time`
and `document_composite::Refreshed`. Rather than gate those (which would break
the documented CPU entry point in a CPU-only build), move the enum to
`src/backend.rs`:

- `lib.rs`: `mod backend; pub use backend::Backend;` — always present, so
  `pictura_render::Backend` keeps working in both builds. This is exactly the
  name the app and the render tests already use.
- `gpu/mod.rs`: `pub use crate::backend::Backend;` — `pictura_render::gpu::Backend`
  keeps resolving for a `gpu` build, and `gpu_filter`'s `use crate::gpu::Backend`
  is therefore untouched.
- `composite_request.rs` and `document_composite.rs` take the type from
  `crate::Backend` instead of `crate::gpu::Backend`.

Alternative considered: keep `Backend` in `gpu` and re-export it unconditionally
from a gated module. Rejected — a `pub use` of a gated module is itself gated;
the type would vanish from CPU-only builds and take `composite()` with it.

### D3 — `composite_request` keeps its signature; the GPU attempt is gated

`CompositeRequest { gpu_enabled }` and both constructors stay. The GPU branch
becomes

```rust
#[cfg(feature = "gpu")]
if request.gpu_enabled && gpu::gpu_available() { ... }
```

so a `gpu_enabled: true` request in a CPU-only build returns `Backend::Cpu` from
the CPU oracle — the same behaviour as a machine without an adapter. `gpu_enabled`
still describes intent, not capability, which is how the current API already
reads.

### D4 — `filter.rs` keeps one call site through a tiny cfg'd helper

`apply_filter_region`'s match arm called `gpu_filter::apply_filter_active`
directly. It now calls `apply_kernel_filter(filter, buf, gpu)`, defined twice:
the `gpu` version delegates to `gpu_filter::apply_filter_active`, the CPU-only
version ignores the flag and calls `pictura_filters::apply` — the exact fallback
`apply_filter_active` already performs. One call site, no public change;
`apply_filter_active` and `filter_gpu_available` themselves are gated with the
module they live in.

### D5 — `metrics` keeps its shape; only the `GpuError`-typed items are gated

The app reads `metrics::counters()`, `take_largest_canvas`, `take_device_ms` and
`enabled()`, and the JSON snapshot iterates `declines` / `layer_declines`. Those
stay unconditional with the same fields, so the app needs no change and a
CPU-only build reports zero declines rather than a different JSON shape.
`reason_index`, `count_decline` and `count_layer_decline` take a `GpuError`, so
they — and the `use crate::GpuError` — are `#[cfg(feature = "gpu")]`. Their only
callers are in `gpu/`, which is gated too.

`DECLINE_REASONS` and `decline_count` stay unconditional: they are public and
name strings only, and the CPU build can still answer "how many declines" with
zero.

### D6 — GPU tests compile only with the feature and still fail without a device

Two shapes, chosen per file:

- **GPU-only binaries** get a crate-level `#![cfg(feature = "gpu")]`:
  `tests/gpu_parity/main.rs` (the parity binary), `tests/gpu_filter_parity.rs`,
  `examples/gpu_text_gamma_bench.rs`, `examples/resident_upload_profile.rs`. With the
  feature off they build to an empty test binary; with it on they run exactly as
  before.
- **Mixed files** gate only the GPU functions: `tests/oracle/main.rs`'s two ignored
  GPU tests, the `composite_gpu` tests in
  `src/tests/{composite,text_gamma,vector_fill}.rs`, the
  `assert_gpu_accepts_and_matches_cpu` helper together with all 32 of its call
  sites across `src/tests/layer_effects.rs` and `src/tests/layer_effects/*.rs`,
  and `src/metrics.rs`'s three decline tests. The files whose only GPU contact
  is an availability probe make that probe feature-aware instead:
  `src/tests/region.rs`, `src/tests/composite.rs`'s
  `proxy_less_type_layer_composites_live_coverage`, `tests/cold_engine_profile.rs`
  and `examples/selection_move_perf.rs`.

None of the `#[ignore]`d device tests changes its body or its ignore reason, and
`require_gpu()` keeps asserting, so a `gpu` build without a device still fails
rather than passes. This is the behaviour `docs/dev/testing-conventions.md`
already specifies, and `require_gpu_panics_without_a_device` proves it: it calls
`require_device(false)` and asserts the panic message, so the fail-not-skip
contract is a runnable check rather than a manual observation.

`tests/backend_pin.rs` and `tests/backend_cpu.rs` are device-free but GPU-typed
(`GpuBackend`, `set_backend`, `selected_backend`, `gpu_available`). They get the
feature gate too, and the `gpu` nextest profile's filter gains
`binary(backend_pin) | binary(backend_cpu)` so they keep running in the
`gpu-parity` job. The Windows renderer step keeps its library run feature-less —
the WARP device cannot survive the full compositor validation, so the gated lib
tests must stay out of it — and compiles the two integration binaries with
`--features gpu` alone. `software-gpu` must not be used there: it makes the probe
accept WARP's CPU-type device, and WARP then loses it mid-validation, which is
exactly why those tests assert the no-acceptable-device path. With `gpu` alone
they keep the pre-change device-free behaviour (the first CI run with
`software-gpu` failed `late_pin_is_refused_after_resolution` this way). Nothing
is dropped, only moved to a command that compiles the feature.

### D6a — Feature unification, and which build proves the win

Cargo unifies features per build, so `cargo nextest run --workspace` builds the
renderer with `gpu` because `pictura-app` asks for it. The CPU-only graph is
therefore proven by the crate-scoped commands, and CI already has one:
`cargo nextest run -p pictura-render` (the step named "Renderer tests without the
oracle feature"). The `pictura-codec` test graph excludes `pictura-app` too, so
`cargo test -p pictura-codec` exercises the CPU-only renderer; its tests name no
GPU symbol.

### D7 — `pictura-app` opts in twice

The normal `pictura-render` dependency gains `features = ["gpu"]`. The
dev-dependency stays `features = ["test-oracle"]` alone: `pictura-app`'s
`build_config::view_oracle` guard asserts that exact line to keep the view-oracle
capability dev-only, and Cargo unifies the two declarations when it builds the
package's test targets, so a test build still gets `gpu`
(`cargo check -p pictura_app --all-targets` compiles the `pictura_render::gpu_available`
calls in `tests_profiles` and `tests_impl`, which only type-check with the
feature on). The app is the end-user build, so its GPU paths — `src/gpu.rs`, the
interop probe, the stroke and composite calls — must keep compiling and working.
This is also what makes `cargo build -p pictura_app` an honest acceptance check.

### D8 — `software-gpu` implies `gpu`; the profile documents it

`software-gpu = ["gpu"]`, and `.config/nextest.toml`'s comment — which already
prints the `gpu-parity` command — now says the flag implies `gpu` and names
`--features "gpu software-gpu"` as the explicit equivalent. The existing
`--features software-gpu` invocations in `.github/workflows/ci.yml`
(`gpu-parity`, the lint step) need no change; the Windows step gains the flag on
its integration-binary command.

### D9 — `locks.rs` loses its caveat

The comment said the predicates live in `pictura-core` because `pictura-paint`
must not depend on this crate, "which pulls in wgpu". The crate no longer pulls
in wgpu by default, but the predicates still belong in `pictura-core` (they are
layer-model predicates, not renderer ones), so the sentence becomes
feature-agnostic: the definitions live in the core crate because they are
layer-model predicates shared with `pictura-paint`.

### D10 — Release valve

If any single item cannot be gated without changing its signature, it stays
unconditional and is listed in the PR body. The expected candidates were
`Backend` (resolved by D2) and the composite request (resolved by D3); if the
build surfaces another, the design narrows to a partial gate rather than a
broken API.

## Measurements

### Before

Taken on the worktree at `origin/master` (515ab3a0) before any edit.

| what | value | how |
|---|---|---|
| distinct packages in `cargo tree -p pictura-render -e normal` | 138 | `cargo tree ... \| sed 's/ (\*)$//' \| sort -u \| wc -l` |
| packages in the `wgpu` closure, `Cargo.lock` | 103 entries / 101 names of 250 | script over the lock's dependency edges |
| cold `cargo check -p pictura-render` | 17.0 s wall | empty `CARGO_TARGET_DIR`, through `scripts/with-build-lock.sh` |

### After

Taken on this worktree after the implementation, same method.

| what | before | after | delta |
|---|---|---|---|
| distinct packages in `cargo tree -p pictura-render -e normal` | 138 | 98 | −40 packages |
| `wgpu` names in that graph | 12 | 0 | −12 |
| packages in the `wgpu` closure, `Cargo.lock` | 103 entries / 101 names of 250 | 103 entries (unchanged; still only reached with `gpu`) | 0 in the lock, −103 in a default build |
| cold `cargo check -p pictura-render` | 17.0 s wall | 13.0 s wall | −4.0 s (−24 %) |
| packages in `cargo tree -p pictura-render -e normal --features gpu` | 138 | 138 | 0 (the GPU build is unchanged) |

The lockfile keeps the closure because an optional dependency still resolves;
the win is that a default build never compiles it. The single cold-check pair is
one warm-cache-cold-target measurement each, not a benchmark series.

Gateway results on this worktree:

- `cargo check -p pictura-render` and `--features gpu`: clean.
- `cargo clippy -p pictura-render --all-targets -- -D warnings`, with and
  without the feature: clean.
- `cargo test -p pictura-render`: 1809 passed, 6 ignored.
- `cargo test -p pictura-render --features gpu --lib`: 1910 passed, 18 ignored
  (the device suites).
- `cargo build -p pictura_app`: succeeds, still linking wgpu.
- `cargo check -p pictura_app --all-targets`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `bash scripts/verify-fast.sh`: fmt, clippy, 6761 tests, file size, archive,
  guard, test plan, parity, source fragments and OpenSpec all pass. The one
  failing check is `shell-seams`, whose `crates/pictura-app/cpp/frame.h` is 804
  lines against a pinned 760-line ceiling; both the file and the ceiling are
  byte-identical to `origin/master` and the gate fails the same way on a
  pristine export of the base commit, so it is not this change.
- CMake configure and full build of the C++ shell: succeed, and
  `./build/pictura --headless --self-test` reports 467 passed, 0 failed.
