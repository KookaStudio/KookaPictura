# Design: selection-marquee-feedback

## Context

See `proposal.md` — Why. The change refines the marquee geometry and the Alt
handling from the archived `selection-marquee-geometry` work, and isolates the
Elliptical Marquee's options. The current code is:

- `pictura_select::geometry::marquee_rect` (Fixed Ratio uses the larger drag
  dimension; Fixed Size centres on `from`).
- `MarqueeToolHandler::trackAltEdge` (first Alt during a drag arms Subtract).
- One shared `SelectionOptions` for both marquee tools.
- `PolygonalLassoToolHandler` uses `kPolygonCloseRadius` in document pixels.

The main specs also contain two requirements the new Alt rule must update:
"Selection combine modes" and "Marquee modifier constraints and size readout".

## Goals / Non-Goals

**Goals:**

- Fixed Ratio stays inside the cursor's non-degenerate drag box; one-axis drags
  derive the other axis.
- Fixed Size's top-left follows the pointer.
- Alt at press subtracts; any Alt edge during the drag mirrors.
- The Elliptical Marquee owns its whole options group, persisted separately.
- The Polygonal close/double-click radius is screen-fixed at 6 px.
- Update the conflicting main-spec requirements through deltas so the merged
  spec has one Alt rule.

**Non-Goals:**

- Changing the selection rasteriser, feather, combine formulas, or history
  states.
- Changing the Magnetic Lasso (it already uses the screen-fixed radius).
- Adding new tool options or non-pixel units.

## Decisions

### D1. Fixed Ratio fits the non-degenerate drag box

```text
width = |dx|, height = |dy|, ratio = w/h
if ratio > 0:
  if width <= 0:       width = height * ratio
  else if height <= 0: height = width / ratio
  else if width / ratio <= height: height = width / ratio
  else:                width = height * ratio
left/top remain anchored at the press point.
```

The selection never exceeds the drag box on a two-axis drag. A one-axis drag is
degenerate, so the derived axis may extend outside it. `f64::round` is half away
from zero, matching `qRound`.

Alternative considered: put the far corner exactly under the cursor. Rejected
because the ratio would then have to move the anchor; the user confirmed the
box-fit behavior.

### D2. Fixed Size's top-left follows the pointer

```text
left = round(to.x), top = round(to.y), width/height = entered size
```

The press point is only the start of the drag; the live preview and the release
both use the current pointer. The rasteriser clips a selection that leaves the
canvas.

### D3. Alt edge rule

`onPress` records `altWasDown_`; it no longer needs `altArmed_`. On any rising
Alt edge during the drag, `mirrorAlt_ = true`. Alt held at press still chooses
Subtract through `resolveSelectionMode`; Alt first pressed during the drag
mirrors immediately. Shift and the combine mode stay captured at press; the
mid-drag mirror changes only the geometry.

### D4. Elliptical Marquee owns its options group

`ToolController` creates a second `SelectionOptions` for `EllipticalMarquee`.
The options bar's `selectionGroupFor(id)` already resolves the per-tool group,
so Style, ratio, size, combine, feather, tolerance, and magnetic fields become
independent. The session store gains an `ellipticalSelection` key; a missing key
restores the defaults (Normal, 1:1, 100×100).

Alternative considered: isolate only the Style/ratio/size fields. Rejected
because it would require a second options type and split the options bar's
binding; the user chose the whole group.

### D5. Polygonal close radius is screen-fixed

`radius = kPolygonCloseRadius / max(zoom, 1e-6)`, matching the Magnetic Lasso.
Both the close click and the double-click use it. The `1e-6` floor prevents a
zero/negative zoom from making every click a close.

### D6. Spec deltas update the conflicting requirements

The `shape-selection-tools` delta modifies "Rectangular and elliptical
marquee", "Selection combine modes", and "Marquee modifier constraints and size
readout". The `selection-tools` delta modifies "Marquee Alt press and mid-drag
behavior".
The `ui/tool-options-persistence` delta adds the independent marquee options.
This keeps the merged spec free of the old Alt-at-press from-centre rule and the
old modifiers-captured-at-press contradiction.

## Risks / Trade-offs

- **Behavior change for Fixed Ratio/Size** → the Rust and Qt tests pin the new
  geometry; the smoke test is the acceptance check.
- **Alt semantics change** → the Qt Alt suite and the self-test cover press,
  mid-drag, second press, and no-selection cases.
- **Second options group** → persistence gains a key; a missing key restores
  defaults. The existing session-store tests stay green.
- **Zoom floor** → the Polygonal behaves like the Magnetic; no negative/zero
  zoom path makes every click a close.

## Migration Plan

No data migration. The new `ellipticalSelection` key is optional; an existing
session without it restores Normal Style and the default ratio/size. Rollback is
the previous build; the session store tolerates the extra key.
