# Design — Maximum and Minimum preserve and radius

## Context

`Filter::Maximum { radius }` and `Filter::Minimum { radius }` are separable
square-footprint morphology over the planar 8-bit buffer
(`crates/pictura-filters/src/other.rs`, `pictura-core::mask_ops`). The catalogue
rows expose one integer slider (`1..=100`) and the GPU plan accelerates the
square footprint up to radius 100.

Modern Photoshop adds a `Preserve` menu. Public descriptions and the issue's
capture agree that `Squareness` keeps square corners and `Roundness` rounds
them, and that `Squareness` is the default form. The project's CS6 contract
predates that menu, so this change records it as an explicit project extension,
not as CS6 parity.

## Goals

- Radius `1..=500`, clamped at 500, with no error above the clamp.
- `Preserve: Squareness` is the default and reproduces the current output bit
  for bit.
- `Preserve: Roundness` uses a disk footprint.
- The dialog warns above 100 px without blocking.
- Existing golden digests and ImageMagick square oracle rows stay valid.

## Non-goals

- Changing the default footprint (that would churn every golden digest).
- A GPU disk kernel; the CPU is the oracle and the round path is opt-in.
- Sub-pixel radii; the engine radius stays an integer and the dialog shows whole
  pixels.

## Decisions

### The choice lives on the filter variant

`Filter::Maximum` and `Filter::Minimum` gain a `MorphologyPreserve` field rather
than a parallel filter kind. The enum is defined in `pictura-filters` and
exported beside `Quality` and `NoiseDistribution`:

```rust
pub enum MorphologyPreserve {
    Squareness,
    Roundness,
}
```

The catalogue `Preserve` control is a `FilterControl::Choice` with the choices
`["Squareness", "Roundness"]` and value `0.0`, so the dialog and the slot
builder share one table. The slot order is Radius then Preserve, matching the
dialog's top-to-bottom order.

### The disk kernel is a row-segment decomposition

A disk is the union of horizontal segments: at vertical offset `dy` the segment
has half-width `hw(dy) = floor(sqrt(r² - dy²))`. The kernel therefore runs
`2r+1` horizontal sliding-max passes, one per `dy`, and folds each pass into the
output:

```text
out(x, y) = combine over dy in [-r, r] of
            max over dx in [-hw(dy), hw(dy)] of
            src(clamp(x + dx), clamp(y + dy))
```

Each pass is the existing monotonic-deque sliding window, so a pass costs
`O(w·h)` and the whole kernel costs `O(r·w·h)` with no radius-sized allocation.
Rows are processed in parallel. The square path keeps the existing separable
`O(w·h)` kernel and is unchanged.

This is slower than the square path by design, which is exactly what the
above-100 warning tells the user. The disk is an inference from public
descriptions; Adobe's exact kernel is closed, so the spec records the footprint
formula as the project's approximation.

### Radius clamp

`MAX_RADIUS` rises to 500 in `other.rs`. The clamp stays a silent `min`, so a
script or a stored Smart Filter with a larger value behaves as 500, as the
current 100 clamp does.

### GPU

`gpu_filter/plan.rs` builds a `Plan::Morph` only for `Squareness`; `Roundness`
returns `None`, so `apply_filter_active` falls back to the CPU oracle. The
square plan clamps to the shared `MAX_MORPHOLOGY_RADIUS` (500), so a scriptable
radius above the ceiling produces the same picture on both backends. The shader
loops over the support and the submit budget chunks large jobs, so radii above
the old 100 cap stay accelerated.

### Warning

The rule lives in the catalogue so it is testable in Rust and not duplicated in
C++:

```rust
impl FilterSpec {
    pub fn warning(&self, params: &[f64]) -> Option<&'static str>;
}
```

`maximum` and `minimum` return a message when slot 0 is above 100. A bridge
function `filter_param_warning(kind, params)` returns it as a `QString`. The
shared dialog owns a hidden `filterWarning` label, updates it on every control
change and when it is prefilled, and shows it non-modally. No `QMessageBox`, so
dragging the slider never interrupts the preview.

## Risks

- The round path is slow at large radii. The warning is the mitigation and the
  square default keeps ordinary use fast.
- A stored action or Smart Filter with one slot (the old shape) now has the
  wrong arity. The engine only receives slot lists from the current descriptor,
  and Last Filter keeps the full list, so the app never sends the old shape; the
  descriptor's arity test covers the refusal.
- The disk formula differs from Adobe's closed kernel at some radii. It is
  marked as an approximation, not parity.
