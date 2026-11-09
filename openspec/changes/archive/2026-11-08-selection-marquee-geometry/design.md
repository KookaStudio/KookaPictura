# Design: selection-marquee-geometry

## Context

See `proposal.md` — Why. The C++ shell owns three pieces of pure geometry:
`marqueeDragRect` in `crates/pictura-app/cpp/selection_geometry.cpp`, the
close-radius `std::hypot` test in `tool_selection.cpp` and
`tool_magneticlasso.cpp`, and `clipSelectionShapeToCanvas`. The Qt layer keeps
the event, modifier, style, and cursor policy.

`pictura-select` already depends on `pictura-core`, and
`pictura-core::shape::drag_rect` already implements the Normal / Shift /
mirror / Fixed Ratio box. `drag_rect` is private today.

## Goals / Non-Goals

**Goals:**

- One Rust implementation of the marquee rect and the close-radius test, with
  unit tests for the cases the C++ self-test and Qt tests do not cover.
- Keep the Qt call sites and the existing tests unchanged.
- Preserve the exact rounding and anchoring of the current C++ implementation.

**Non-Goals:**

- Porting `clipSelectionShapeToCanvas`. It needs a real even-odd polygon
  boolean; no boolean-geometry crate is in the workspace. It stays in C++ and
  a follow-up issue tracks the port.
- Changing preview performance, cursor policy, or selection behaviour.
- Retiring `tsc_marquee_geometry` (it stays as a parity check).

## Decisions

### D1. Geometry lives in `pictura-select`, reusing `pictura-core::shape::drag_rect`

`pictura-select` is the selection-math crate and already depends on
`pictura-core`. `drag_rect` becomes `pub` so the marquee maps onto it:

| C++ case | Implementation |
|---|---|
| Normal | `drag_rect(from, to, None, mirror)` |
| Shift | `drag_rect(from, to, Some(1.0), mirror)` |
| Fixed Ratio | the deleted C++ branch, kept verbatim so the width and anchor round exactly |
| Fixed Size | inline in `marquee_rect`, centred on the press point |

`drag_rect` keeps its non-positive-aspect guard for the shape tools; the Fixed
Ratio branch keeps the same `ratio > 0.0` guard. The returned floats are
rounded with `f64::round`, which is half-away-from-zero like Qt's `qRound`.

Alternative considered: map Fixed Ratio onto `drag_rect` too. Rejected because
`drag_rect` recomputes the width from the constrained height, which can shift
the rounded width and anchor by one pixel for non-power-of-two ratios; the
Fixed Ratio branch keeps the old arithmetic.

### D2. Fixed Size is computed inline in `marquee_rect`

`qRound(a.x()) - w / 2` uses integer division, so an odd width truncates
toward zero. `marquee_rect` computes Fixed Size inline and gets a unit test
with an odd size.

### D3. One `within_radius(a, b, radius)`, radius as a parameter

The Polygonal Lasso compares document distance; the Magnetic Lasso compares
`distance * zoom`, so its effective document radius is `radius / zoom`. The
shared function compares squared distances and takes the radius as an argument,
so both callers keep their own policy and neither needs `sqrt`.

Alternative considered: pass `zoom` into the Rust function. Rejected because
zoom is Qt view state, not selection geometry.

### D4. The existing selection bridge, with the C++ header kept as the adapter

`cxxqt_object/impl_selection/bridge.rs` gains the two free functions (the
selection bridge already owns the selection-move and Quick Selection calls): `selection_marquee_rect(...) -> Vec<i32>` and
`selection_within_radius(...) -> bool`. The C++ `selection_geometry.{h,cpp}`
keep their signatures and become thin adapters, so `tool_selection.cpp`,
`tools_marquee.cpp`, and `image_view.cpp` do not move.

Flat `Vec<i32>` matches the house bridge style (`magnetic_trace`,
`transform_api.cpp`). The marquee preview is one rectangle per pointer move;
one small allocation is not material next to the existing Qt work.

Alternative considered: return `[i32; 4]`. It avoids the allocation but breaks
the flat-`Vec` convention; the allocation is not worth a special case.

### D5. `clipSelectionShapeToCanvas` is deferred

`QPainterPath::intersected` is an even-odd boolean, not a rectangle clamp. The
pinned self-overlap case cannot be reproduced by Sutherland–Hodgman without
bridge edges, and adding `i_overlay` is a new dependency for one function. The
function stays in C++ unchanged; the follow-up issue records the options
(`i_overlay`, a hand-written arrangement clipper) and the differential-corpus
plan.

### D6. Delete `dragRect`

It has no callers outside `marqueeDragRect`. The public Rust function replaces
it; leaving a second C++ entry point would keep a second untested copy.

## Risks / Trade-offs

- **Rounding drift between Qt and Rust** → the Rust tests pin the expected
  integer rects and the Qt tests plus `tsc_marquee_geometry` stay green.
- **Fixed Size integer truncation** → explicit odd-size test.
- **Bridge allocation per pointer move** → one `Vec<i32>` of four ints; the
  preview already rebuilds Qt geometry per move. Revisit only if profiling
  shows it.
- **Clip remains C++-only** → the issue's item 3 is tracked by a follow-up;
  no behaviour is weakened.
- **`drag_rect` becomes public API** → it is documented and already the single
  source of the shape drag math; exposing it removes a copy rather than adding
  a second consumer with different semantics.

## Migration Plan

Land the Rust module and tests first, then switch the C++ adapters in the same
change. No data migration or rollback path is needed: the change is
behaviour-preserving and the C++ entry points keep their signatures.
