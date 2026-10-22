# M6-C — Filter Integration

Goal: make M6 filters usable — apply a destructive filter to a pixel layer,
confined by the active selection, and expose it in the app.

Depends on M6 (`pictura-filters`) and M5-C (selection → mask). Spec:
`docs/06-filters/filters-overview.md`; selection confinement per
`docs/08-selection/selection-model.md`.

## Scope

In:
- `pictura-render`: `apply_filter(layer, filter, mask)` — destructive filter
  application to a pixel layer's color channels, gated by a document-coordinate
  coverage mask. New dependency: `pictura-filters` (same reason render already
  depends on `pictura-adjust`).
- `pictura-app`: a `PictureView::apply_filter(kind)` command that applies a
  filter to the topmost pixel layer with the active selection as the mask,
  recomposites, and emits `changed`; a filter combo + "Apply Filter" button in
  the dock; headless `--self-test` proving confinement.

Out (later): non-destructive Smart Filters, filter dialogs/preview, per-layer
filter targeting, undo, filters on adjustment/group layers, 16/32-bit.

## Contract (authoritative)

```rust
// crates/pictura-render
/// Apply a destructive `filter` to a pixel layer's color channels (0,1,2),
/// gated by an optional document-coordinate coverage `mask` (`None` = full
/// frame). The transparency channel (`-1`) is never modified.
pub fn apply_filter(
    layer: &mut pictura_core::Layer,
    filter: &pictura_filters::Filter,
    mask: Option<&pictura_core::LayerMask>,
) -> Result<(), pictura_filters::FilterError>;
```

Semantics:
- `lw = rect.width()`, `lh = rect.height()`; if either is `<= 0` → `Ok(())`.
- Extract the color channels into a planar `PixelBuffer` of size `lw × lh × 3`.
  A color layer contributes `0,1,2`; a Grayscale layer (only channel `0`
  present) has channel `0` replicated across the three planes and the filtered
  plane written back to channel `0`. A data length `!= lw*lh`, a missing
  channel `0`, or a mix of present/missing `1`/`2` → `FilterError::InvalidParams`.
- Apply the filter to that buffer (filter validation still applies).
- Gate back per local pixel `(lx,ly)` at document coords
  `(rect.left+lx, rect.top+ly)`: `coverage` = the mask value there (outside the
  mask rect use `mask.default_color`) or `255` when `mask` is `None`; write
  `round(orig + (filtered − orig) * coverage/255)` into channels `0,1,2`, or
  into channel `0` alone for a Grayscale layer.
- Filter channel `-1` (transparency) with the same kernel when the layer is not
  transparency-locked, writing the masked result back too; a transparency lock
  keeps `-1` bit-identical and leaves pixels whose alpha is 0 untouched. Do not
  change `rect`, `mask`, opacity, or blend.

`apply_filter_region(layer, filter, mask, gpu_enabled, region)` applies the same
filter over a document rect clamped to the layer rect, leaving every pixel
outside it unchanged; a neighborhood filter reads up to its support outside the
rect, where the source is clamped. `filter.preview_apron()` returns a conservative
support margin. A filter-dialog preview passes the visible viewport rect expanded
by that apron, so dragging a control costs at most a viewport while the visible
area stays exact; OK still filters the whole layer.

```rust
// crates/pictura-app (PictureView qobject)
/// Apply a filter `kind` to the topmost pixel layer, confined by the active
/// selection, then recomposite. Returns false when there is no document, no
/// filter matches `kind`, or there is no pixel layer.
fn apply_filter(mut self: Pin<&mut Self>, kind: &QString) -> bool;
```

Filter kinds → `Filter` (defaults chosen so a fresh apply visibly changes a
non-trivial image): `gaussian-blur` (radius 5), `box-blur` (radius 3),
`motion-blur` (angle 0, distance 15), `median` (radius 2), `despeckle`,
`sharpen`, `sharpen-more`, `unsharp-mask` (amount 150, radius 1, threshold 0),
`add-noise` (amount 25, Uniform, per-channel, fixed seed 1). Unknown → `None`.

"Topmost pixel layer" = the last layer in `doc.layers` (bottom-first order)
with `adjustment.is_none() && !is_group`.

## Task DAG

| ID | Task | Owner | Owns |
|---|---|---|---|
| M6C-A | `pictura-render::apply_filter` + tests | agent | `crates/pictura-render/src/**`, `crates/pictura-render/Cargo.toml` |
| M6C-B | App command + dock + self-test | agent | `crates/pictura-app/**` |
| M6C-C | OpenSpec change + verify | orchestrator | `openspec/**`, `docs/dev/**` |

## Exit gate

- `cargo test --workspace` green (render `apply_filter` unit tests + app self-test).
- `xvfb-run ./build/pictura --self-test <layered.psd>` proves a filtered change
  confined to the selection (pixels outside the selection unchanged).
- fmt/clippy clean; `scripts/guard.sh` green; `openspec validate --all --strict`
  green with the new `m6-filter-integration` change.
