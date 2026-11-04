# Design

## Context

See `proposal.md` for motivation. The current seam is spread across:

- `composite.rs`: `composite_rgba` (CPU whole-document oracle, banded when large).
- `composite_region.rs`: `composite_rgba_region` (raw CPU region; no clamping).
- `gpu/mod.rs`: `composite_gpu`, `composite_region_active`, `composite_gpu_or_cpu`,
  `composite_active`, plus the private `composite_gpu_region`.
- `composite_native.rs`: `composite_native` and `refresh_native_composite`.
- `render_session.rs`: `RenderSession::preview`, which composites a reduced
  document at a pyramid level and is CPU-only.

`composite_active` and `composite_region_active` are the application's real
entry points, and `composite_active` is also called by the public render
operation `document_ops::translate_layer_active`. `composite_gpu_or_cpu` has no
production caller. `composite_rgba_region` has no cross-crate production caller,
and `composite_native` has no cross-crate caller at all. `composite_gpu` is the
GPU oracle used by the external `gpu_parity` suite to assert typed `GpuError`
values. `RenderSession::preview` is a separate preview-cache concern, not part of
the `(doc, region, backend)` cross-product.

## Goals / Non-Goals

**Goals:**

- One policy entry point: `composite(CompositeRequest) -> (PixelBuffer, Backend)`.
- Rectangle clamping and the GPU-to-CPU fallback live inside that entry point.
- The application no longer chooses a composite function or a fallback path.
- Byte-for-byte behavior is unchanged for the CPU, region, and clamping cases;
  GPU output keeps the existing ±1 LSB parity.

**Non-Goals:**

- Changing the GPU eligibility rules, the blend math, the plan, the banded
  compositor, or the render-session preview cache.
- Folding the source-depth (`Samples`) output into the request. It returns
  `PixelBuffer` only.
- Folding `RenderSession::preview` into the request.
- Adding a new dependency or a new backend selection mechanism.
- Updating `docs/GLOSSARY.md` in the same commit. Its entries for the removed
  and demoted functions are updated by task 5.1 in a separate commit because
  `docs/` changes need `TASK-ALLOWS-DOCS`.

## Decisions

### One request struct, one dispatch function

Add `crates/pictura-render/src/composite_request.rs`:

```rust
pub struct CompositeRequest<'a> {
    pub doc: &'a Document,
    pub region: Option<PsdRect>,
    pub gpu_enabled: bool,
}

impl<'a> CompositeRequest<'a> {
    pub fn whole(doc: &'a Document, gpu_enabled: bool) -> Self;
    pub fn region(doc: &'a Document, rect: PsdRect, gpu_enabled: bool) -> Self;
}

pub fn composite(request: CompositeRequest<'_>) -> (PixelBuffer, Backend);
```

`composite` normalizes `region: None` to the whole-document rectangle, clamps a
`Some` rectangle to the document bounds, and returns a zero-dimension buffer with
`Backend::Cpu` when the clamped intersection is empty. It then:

1. If `gpu_enabled` and `gpu::gpu_available()`, call the internal GPU region
   kernel. On `Ok`, return `(buffer, Backend::Gpu)`.
2. Otherwise, or on any `GpuError`, call `composite_rgba` for a whole-document
   request or `composite_rgba_region` for a sub-region, and return
   `(buffer, Backend::Cpu)`.

The whole-document branch deliberately calls `composite_rgba` rather than the
region path so the banded compositor keeps its existing entry. The region branch
reuses `composite_rgba_region` unchanged, so its bytes are exactly the old
`composite_region_active` CPU fallback.

Why a new module rather than adding to `composite.rs`: `composite.rs` is already
past the 800-LOC target and the request spans the CPU and GPU modules. A small
module keeps the dispatch seam explicit and avoids a circular module read.

### `gpu_enabled: bool`, not a request-side `Backend` enum

The issue names a `backend` field. Every application call site already has the
`gpu_compute: bool` preference, and the existing functions take a bool. A
request-side enum (`BackendPolicy { Auto, Cpu }` or `Backend { Gpu, Cpu }`) would
add a conversion at roughly 70 call sites and would make `Backend::Gpu` mean
"try the GPU" while the return value means "the GPU ran". The bool keeps the
request honest and the return `Backend` unambiguous.

Alternative considered: reuse `Backend` as the request type. Rejected because
the same value would mean both the request and the result, and a `Gpu` request
can still return `Cpu`.

### Keep two oracles public, demote the rest

The stable public surface becomes:

- `composite` and `CompositeRequest`: the policy entry point.
- `composite_rgba`: the CPU oracle, still used by application paths that refresh
  the authoritative `doc.composite` and by tests.
- `refresh_native_composite`: the application-facing source-depth refresh.
- `gpu_available`, `Backend`, `GpuError`, and the existing backend-selection
  functions.

Demoted to `pub(crate)`:

- `composite_rgba_region` (only in-crate and test callers after the rewiring).
- `composite_gpu` (the GPU oracle; test-only `#[cfg(test)] pub(crate)`, because no
  production caller remains and in-crate tests keep using it).
- `composite_native` (no cross-crate production callers; `apply_image`,
  `document_ops/depth`, and the in-crate `src/tests/native_depth.rs` use it).

Removed:

- `composite_active`
- `composite_region_active`
- `composite_gpu_or_cpu`

`RenderSession::preview` is unchanged. It is a preview cache over a reduced
document and does not select a backend.

### GPU parity tests move their error assertions in-crate

Demoting `composite_gpu` means the external `tests/gpu_parity` binary can no
longer call it. The split:

- Numeric parity tests keep running in `tests/gpu_parity` and use
  `composite(CompositeRequest::whole(doc, true))`, asserting `Backend::Gpu`
  before comparing bytes. `require_gpu()` already calls `gpu_available()`
  (`tests/gpu_parity/common.rs`), so it needs no change.
- Assertions on specific `GpuError` variants move to
  `crates/pictura-render/src/gpu/oracle_tests.rs`. Those tests are in-crate, so
  the test-only `#[cfg(test)] pub(crate) composite_gpu` is visible. The request
  matrix test lives in
  `crates/pictura-render/src/gpu/request_tests.rs`. The GPU nextest profile
  already selects `test(/^gpu(_filter)?::/)`, so both modules still run in the
  `gpu-parity` CI job. Device-free declines (Dissolve, unsupported adjustment)
  keep running in the default job; only
  `document_past_2d_product_limit_returns_too_large` keeps its `#[ignore]`
  reason string.
- The diverging-device and stroke-fallback scenarios are marked `manual:` in the
  test plan: a device whose validation scene diverges cannot be forced on
  lavapipe or WARP, and adding a production test hook would exceed this change.

This is the main cost of demoting `composite_gpu`. The alternative, keeping it
public as an oracle, would leave one extra public entry point and fail the
issue's "six superseded" acceptance.

### `composite_native` becomes internal

`composite_native` has no cross-crate caller. `apply_image` and
`document_ops/depth` use it inside the crate. The exposed application operation
`refresh_native_composite` stays public. The native-depth spec is updated to
describe the source-depth composite as internal and the refresh as exposed.

### Application call-site mapping

- `composite_active(doc, gpu).0` becomes
  `composite(CompositeRequest::whole(doc, gpu)).0`.
- `composite_region_active(doc, rect, gpu).0` becomes
  `composite(CompositeRequest::region(doc, rect, gpu)).0`.
- Tests that need the backend use the returned `Backend`.
- Application paths that deliberately compute the CPU oracle for
  `doc.composite` keep calling `composite_rgba`; those are not fallback choices.

## Risks / Trade-offs

- **Region bytes drift** -> the request reuses `composite_rgba_region` unchanged
  and only adds the same clamping `composite_region_active` had. A matrix test
  compares every region to the corresponding whole-document slice.
- **The banded fast path is bypassed** -> the whole-document branch calls
  `composite_rgba`, preserving the existing banded decision. The region branch
  keeps `composite_rgba_region`'s own banded decision.
- **Clamping differs for absurd dimensions** -> use the same saturating
  `min(i32::MAX as u32)` bound as `composite_rgba` so a document wider than
  `i32::MAX` cannot wrap the clamp. This is stricter than the old
  `composite_region_active` and cannot affect real documents.
- **GPU error coverage is lost from the external parity binary** -> the moved
  in-crate tests are selected by the existing `gpu` nextest filter; only the
  `TooLarge` case keeps its `#[ignore]` reason string.
- **Large mechanical test diff** -> keep the rewrite mechanical: constructors
  preserve the old argument order, and no test changes its assertions beyond
  reading `Backend` from the request result.
- **Metrics attribution changes** -> `composite_gpu_region` still counts
  declines; the request calls it only after the availability gate, matching the
  old `composite_active` behavior. The metrics tests stay in-crate.

## Migration Plan

1. Add `composite_request.rs`, declare the module, and make
   `composite_gpu_region` `pub(crate)`; route the request through it.
2. Rewire every caller: render production (`document_ops/crop.rs`), app
   production, in-crate render tests, app tests, and integration tests. Move the
   `GpuError` assertions to `gpu::oracle_tests` and add the request matrix to
   `gpu::request_tests`.
3. Demote `composite_native` and `composite_rgba_region` to `pub(crate)`, and
   make `composite_gpu` test-only (`#[cfg(test)] pub(crate)`); change the
   `composite.rs` `pub use` to `pub(crate) use`; remove them from the crate-root
   re-exports.
4. Delete `composite_active`, `composite_region_active`, and
   `composite_gpu_or_cpu`, plus the now-unused `composite_cpu_region` helper.
5. Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo nextest run --workspace`, the GPU profile where available,
   `bash scripts/verify-fast.sh` (with a CMake build first, or
   `bash scripts/verify-full.sh`), and `openspec validate --all --strict`.
6. Update `docs/GLOSSARY.md` in a separate commit carrying `TASK-ALLOWS-DOCS`
   (task 5.1).
7. Archive the change with `openspec archive composite-request` and run
   `scripts/check-openspec-archived.sh`.

Rollback is a straight revert. Because the public API shrinks only in steps 3–4,
the change can be split into an additive commit (request plus rewired callers)
and a removal commit; reverting the removal commit restores the old entry points
without touching the request.
