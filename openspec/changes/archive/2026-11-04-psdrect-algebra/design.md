# Design: psdrect-algebra

## Context

See `proposal.md` — Why. `PsdRect` is a signed `i32` rectangle in PSD
coordinates; `width`/`height` already saturate. The duplicated helpers fall into
four families: rect intersection, clipping to the document canvas, rect union,
and growing. Their differences are only in the empty/miss representation and in
whether a non-positive grow pad is a no-op.

The change is cross-cutting (five crates) but mechanical; the design decisions
below fix the one shared contract so every deletion is a pure substitution.

## Goals / Non-Goals

**Goals:**

- One documented rectangle algebra on `PsdRect`.
- One return-shape convention for a miss: `Option`.
- Delete the private helpers, not wrap them.
- Keep damage, clipping, and compositing goldens byte-identical.

**Non-Goals:**

- Converting the foreign rectangle types in `pictura-filters`
  (`photorust::pixmap::Rect`, `vanishing_point::canvas::Rect`). They have
  different fields and semantics and are not `PsdRect` copies.
- Converting the `(left, top, right, bottom)` tuple helpers inside
  `layer_effects`; that is a GPU-buffer representation, not a `PsdRect`.
- Not deleting `cxxqt_object::helpers::clamp_region`; it stays as the
  `(x0, y0, w, h)` tuple adapter and delegates to `PsdRect::clip_to`.
- Converting the bounds clamps `impl_filters::clamp_to` and
  `filter_tools/canvas_proxy.rs::clamp`; they clamp to an arbitrary bounds
  rect, not the canvas.
- Converting the remaining inline min/max expressions in `auto_blend/mod.rs`,
  `gpu/clip.rs`, `gpu/mod.rs`, `history/planes.rs`,
  `impl_paint.rs::grow_extent`, and the paint `smudge.rs`/`replace.rs`/
  `focus.rs` grow+clip blocks; they are inline expressions, not private
  helpers.
- Changing any requirement or behavior in `openspec/specs/`.
- Changing `docs/GLOSSARY.md` (a `docs/` edit would need
  `TASK-ALLOWS-DOCS`); it already carries stale line numbers for these
  methods.

## Decisions

### D1. `intersect` and `clip_to` return `Option<Self>`

```rust
pub fn intersect(self, other: Self) -> Option<Self>;
pub fn clip_to(self, width: u32, height: u32) -> Option<Self>;
```

The copies split between bare (inverted on miss) and `Option`. Standardizing on
`Option` makes a miss explicit and prevents a caller from indexing an inverted
rectangle. Bare callers that need a value use `.unwrap_or_default()`; callers
that branch use `?` or `let … else`. `is_empty()` remains available for checks.

Alternative considered: normalize to an empty `Self`. Rejected because it keeps
the "empty is a value" ambiguity the issue is trying to remove, and it would
force the three `Option` call sites to re-derive `Option` manually.

### D2. `clip_to` takes canvas dimensions, not a rectangle or a `Document`

Every existing canvas clamp is to `0..width × 0..height`. `clip_to` converts
the dimensions with `width.min(i32::MAX as u32) as i32` and
`height.min(i32::MAX as u32) as i32`, so the helpers that currently use a bare
`width as i32` become consistent. Taking `(width, height)` deletes the seven
canvas-clamp helpers without every caller constructing a canvas `PsdRect`, and
keeps `PsdRect` free of a `Document` dependency at the call site.

Alternatives considered: `clip_to(bounds: Self)` (callers still build a canvas
rect) and `clip_to(doc: &Document)` (couples the method to document semantics).
Both are workable; dimensions are the smallest deletion.

### D3. `union` is plain min/max, with no empty special case

```rust
pub fn union(self, other: Self) -> Self;
```

All current rect-union copies use plain min/max. Making an empty rectangle the
identity (as `vanishing_point::Rect::union` does) would silently change
damage/history bounds whenever an empty rect is folded in. The method must stay
plain, and a test pins it.

### D4. `grow` saturates and accepts a negative pad

```rust
pub fn grow(self, pad: i32) -> Self; // saturating_sub/add on every edge
```

This matches `layer_effects::grow_rect` and `gpu/clip::grow`. The one helper that
no-ops on `pad <= 0` (`cxxqt_object::helpers::grow_rect`) is only called with
non-negative pads, so deleting the guard is not a behavior change. Saturating is
also safer than `healing::inflate` and `impl_filters::expand`, which currently
use unchecked `+`/`-`; the unified method removes that overflow path.

### D5. Add `Default` for the canonical empty rectangle

`#[derive(Default)]` gives `(0, 0, 0, 0)`, which `is_empty()` reports as empty.
This lets `clip_to`/`intersect` callers write `.unwrap_or_default()` and lets
`canvas_damage::empty()` disappear. Alternative: an associated `EMPTY` const;
`Default` is the idiomatic spelling and costs no extra code.

### D6. Keep the `Option` union combinators

The four `union(Option<PsdRect>, Option<PsdRect>)` helpers are not rect algebra:

- `damage::union` is **None-wins** (an unbounded side makes the edit unbounded).
- `gpu/clip::union`, `healing/stroke::union`, and `liquify::union` treat `None`
  as identity.

They stay as named functions. `damage::union` collapses to
`Some(a?.union(b?))`; the three None-identity combinators keep their `match`
and call `a.union(b)` only in the `(Some, Some)` arm. Merging them into one
helper would lose the None-wins distinction.

### D7. Receivers are by value

`PsdRect` is `Copy`; `self` keeps call chains readable (`a.intersect(b)`,
`rect.grow(pad)`). Existing `width`/`height` keep `&self`.

## Risks / Trade-offs

- **[Inverted misses become `None`/empty]** → Every current bare caller already
  tests `width() > 0`, so results are equivalent. The new core tests cover
  disjoint/touching/inverted inputs, and the existing render/paint/filters/app
  suites cover the goldens.
- **[Fully off-canvas GPU layout changes]** → `gpu/raster::clip_to_document`
  currently keeps an off-canvas `x0` when the rect misses; `clip_to` yields 0.
  The `n == 0` early return makes this unobservable, so no test is added.
- **[Negative grow pad]** → The unified `grow` shrinks; verify every call site
  passes a non-negative pad. The paint/effects call sites derive pads from
  reach/feather and are non-negative.
- **[`lib.rs` file size]** → `pictura-core/src/lib.rs` is 1146/1200 LOC. The five
  methods land around 1181. If the docs push it over, move `PsdRect` and its
  impl to `crates/pictura-core/src/rect.rs` and re-export; do not raise the
  allowlist.
- **[Stale `MODEL.md`]** → Update `crates/pictura-core/MODEL.md` with the new
  methods; it is crate documentation, not `docs/`, so no guard marker is needed.

## Migration Plan

No runtime migration. Land as one refactor commit; rollback is reverting the
commit. Implementation order: add methods and tests in `pictura-core`, then
replace helpers crate by crate (render, app, paint, filters), then run the
existing suites and `verify-fast`.

## Open Questions

None. The one open API choice from exploration (`Option` versus normalized
`Self`) is settled in D1.
