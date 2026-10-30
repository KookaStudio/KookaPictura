# Design

## Context

See `proposal.md` — Why. The constraints that shape the approach:

- `composite_full` (`impl_core.rs:882-903`) runs four whole-document passes:
  composite, `buffer_to_srgb`, `store_composite`, and `ViewPyramid::rebuild`.
  A canvas-sized adjustment preview reaches it through `Edit::Full`
  (`adjustment_edit.rs:110-140`, `damage.rs:96-104`, `edit.rs:300-304`).
- The region path (`composite_region.rs`, `gpu/mod.rs:217-248`) already
  composites an arbitrary document rectangle exactly, but it always starts from
  layer 0.
- `ViewPyramid` stores premultiplied levels below 0 that are box-averages of the
  exact composite. `patch_level` overwrites a stored level, which the stroke
  preview already uses (`impl_paint.rs:721-739`).
- `FilterProxy` reduces a document to a region and scale; adjustment layers
  survive because `composite_adjustment` ignores the layer rect, but channel-less
  fill layers do not: `reduce_layer` empties their rect and the fill functions
  read it (`fill.rs:527-530`, `:555-563`, `:689-692`).
- The compositor accumulator is straight-alpha f32 (`Px`, 16 bytes/pixel). A
  document-sized backdrop is 256 MB at 4000² and 4 GB at 16000².
- Clipping runs are atomic (`composite_clipping.rs:34-44`); a knockout above the
  resume point needs the bottom-layer-alone base (`composite_knockout.rs:37-47`).
- The GPU compositor loops from layer 0 (`gpu/mod.rs:193-210`).
- `ImageView` already knows the presented level and the visible document
  rectangle (`image_view.cpp:469-505`); `preview_present_level` already forces a
  level for stroke previews (`impl_paint.rs:814-820`).
- `compositing/current-picture` says the maintained composite is current after
  every presented preview that updates it; the commit must keep that true.

## Goals / Non-Goals

**Goals:**

- One preview rule that covers both zoom regimes: composite the visible document
  rectangle at the presented level, starting from a cached backdrop below the
  edited layer.
- Keep the preview's memory proportional to what the canvas shows, not to the
  document.
- Keep the exact composite the single source of truth at the commit, with no
  approximate pixels surviving it.

**Non-Goals:**

- A document-sized full-resolution backdrop.
- Changing stroke-frame compositing or the GPU compositor.
- A `RenderSession` type that owns display state; the session owns only the
  backdrop and the reduced preview document.
- Moving the commit to a worker before the profile justifies it.

## Decisions

### The preview frame is the visible rect at the presented level

The session composites only the visible document rectangle (reported by
`ImageView`) at the level the canvas presents. At level 0 that is exact in view;
above 0 it is a reduced-document composite and is allowed to differ from the
level of the exact composite. This is the same contract the stroke preview
already has (`tools/mask-stroke-preview`): present approximately, repair at the
commit.

Alternative rejected: a document-sized backdrop plus a full-resolution preview.
It costs 16 bytes per pixel and buys nothing at 25% zoom, where the canvas shows
1/16 of the pixels.

### The backdrop is cached at the preview's resolution

The backdrop is the composite of the layers below the edited layer over the
preview region at the preview scale. At fit zoom on a 4000² document and level 2
that is 1 MP of f32, about 16 MB; at 100% it is the viewport, also about 16 MB.
Each tick copies the backdrop into a scratch canvas and composites only the
layers at and above the edited layer.

Alternative rejected: caching the compositor accumulator at full resolution. It
is exact but 256 MB at 4000² and useless at 16000².

### The session lives in `pictura-render`; the app owns display state

A new `render_session` module owns the backdrop slot, the prefix digest, the
reduced preview document, and the two composite entry points (preview and exact
refine). `PictureViewRust` owns the session and does the display patching
(`level0` at level 0, `pyramid.patch_level` above it). `pictura-render` gains no
Qt dependency, and the compositor gains one entry point that composites
`doc.layers[k..]` onto a caller-supplied `Canvas`.

Alternatives rejected: an app-only module (the compositor seam still has to
live in `pictura-render`, so the cache would straddle two crates anyway); C++
ownership (splits document and history ownership across the FFI and would keep
stale state across undo, open, and Revert).

### The backdrop is keyed by a prefix digest and validated on read

The key is a digest over the layers below the edited layer in composite order:
rect, blend, opacity, fill, visibility, clipping, knockout, adjustment/effect
parameters, and `Plane::ident()` for every channel, mask, blending range, and
smart-object plane, the document pattern library's identity, plus the document
size, mode, level, and preview rect. `content_key` does not hash the pattern
library, so the digest must add it explicitly. A read hashes the current prefix
and compares; a mismatch drops the slot and rebuilds. There is no invalidation
notification to forget, and a stale backdrop can only be served if the digest is
incomplete.

The digest is the load-bearing risk: it must include every input that can change
the prefix's pixels. Layers whose appearance depends on state outside the prefix
— text, smart objects, pattern-driven effects — contribute their existing
content keys (which already hash the font generation and text state,
`layer_source/key.rs`), and a stack that still cannot be summarised falls back
to the full path.

Alternatives rejected: explicit invalidation messages from every edit site
(easy to miss); a document-wide monotonic revision (invalidates on edits above
the layer, which the spec forbids).

### Resume only at a structurally valid boundary

Resume at root index `k` only when the edited layer is a non-clipped, root-level
layer, the document is RGB at 8-bit, no stroke is live, and any knockout layer
above `k` gets the bottom-layer-alone base (cached with the backdrop). A hidden
edited layer is a valid resume point; the suffix simply omits it. Clipped
targets, nested targets, non-RGB and native-depth documents, and knockout stacks
that cannot supply the base fall back to the full composite. This keeps every
fallback byte-identical by construction.

Alternative rejected: resuming inside a clipping run by reconstructing the
group's isolated canvas. It is possible, but it multiplies the resume states for
a case the issue does not measure.

### Preview requests coalesce in the Properties panel, mirroring the Layers panel

The Properties panel coalesces slider and curve events with a single-shot timer
scheduled for the next event-loop turn (0 ms), so at most one preview composite
runs per presented frame. The Layers panel's 120 ms delay timer
(`layers_panel_preview.cpp:90-127`) is the same pattern with a longer window. The
timer holds the latest key/value or curve and calls `adjustment_set`,
`adjustment_set_curve`, or `adjustment_reset` at most once per frame. The latest
value wins because only the pending value is applied; intermediate values never
reach Rust.

`gestureEnded` is connected directly to `PropertiesPanel::commit`
(`properties_panel.cpp:172`), so `commit()` must flush the pending preview —
applying the latest key/value or curve — before calling `adjustment_commit`;
otherwise a slider release can commit the previous value. After the flush,
`adjustment_commit` runs the exact composite from the document before recording.

Alternative rejected: a new Rust pending-preview token drained through
`flush_present`. It duplicates the timer the Layers panel already proved, adds a
signal and a present path, and cannot be exercised more cheaply than the Qt Test
the existing panel pattern already has.

### The commit is the exact composite

`adjustment_commit` changes from `Edit::Record` to an exact composite followed by
the record. A bounded edit keeps the region path; a canvas-sized adjustment uses
the full composite, because its effect is the canvas. The commit also clears the
display-preview state and resets the pyramid, so every stored level is rebuilt
from the exact level 0. The commit compares the final value with the value
captured at gesture start and records no history state when they are equal, for
adjustments and for opacity/fill round trips.

Alternative rejected: a per-tick off-thread refine. It does the same total work
as today and needs a job, a coalescing queue, and a staleness contract; the
commit already provides the exact frame once per gesture.

### The reduced document is built once per gesture

Level 0 composites the original document's visible rectangle exactly; only levels
above 0 use a `FilterProxy`-style reduction of the visible region at the preview
scale. The reduced document is built on the first preview of a gesture and kept
until the gesture ends, the region/scale changes, or the document changes outside
the preview's own revision bump (undo/redo, an edit above the target). Each tick
sets the edited layer's parameters on the reduced copy and composites it.

`reduce_layer` gains one fix: a channel-less fill layer keeps its rect mapped to
the proxy grid instead of being emptied, so solid, gradient, and pattern fills
survive the reduction. Effects and vector masks are not scaled (the existing
`ponytail:` ceiling); a stack whose visible difference depends on them is
approximate above level 0 and exact at the commit. A stack containing a live type
layer, a proxy-less smart object, or a vector fill cannot be reduced faithfully
and falls back to the exact path.

### Memory and backend fallbacks

The backdrop is admitted only when the backdrop plus the scratch canvas's
worst-case footprint fits a session budget (start at 64 MB, tune with the
profile); otherwise the exact path runs. Previews always composite on the CPU,
because a CPU f32 backdrop cannot seed the GPU pass; the commit uses the
configured backend. An empty visible rectangle is a no-op. No presented level,
an unsummarisable prefix, content that cannot be reduced (a live type layer, a
proxy-less smart object, a vector fill), a soloed mask, a live stroke, a non-RGB
or native-depth document, and a document with no stored level all fall back to
the existing exact path.

## Risks / Trade-offs

- **An incomplete prefix digest serves a stale backdrop** → include
  `Plane::ident()` and every scalar field that affects the prefix, hash the
  font/text/pattern generations, and add a test that mutates each field and
  asserts the digest changes.
- **A visible jump at the commit above level 0** → level-0 previews are exact in
  view; above 0 the commit holds the approximate frame until the exact levels are
  rebuilt, and the profile reports the maximum per-session delta.
- **Consumers read a stale composite during a drag** → the current-picture
  accessor gains the display-preview guard, so a read composites; the commit
  captures only after the exact composite.
- **The reduced document differs beyond the target** → fill rects are fixed; the
  effects/vector-mask ceiling is documented, and a stack containing live type, a
  proxy-less smart object, or a vector fill falls back to the exact path.
- **A coalesced preview is never applied** → the commit flushes the pending
  value before compositing, so the committed frame cannot miss the last event;
  the timer is stopped and cleared on cancel and document replacement, as the
  Layers panel already does.
- **A cancelled preview leaves the approximate frame on screen** → Esc cancels:
  it discards the pending value, restores the pre-gesture display (reset the
  pyramid or repatch the captured frame), and records nothing. A layer switch
  commits the pending value, and a document replacement cancels.
- **Backdrop plus scratch doubles the preview memory** → the budget gate admits
  only when both fit; the scratch is reused across ticks.

## Migration Plan

No persistence or format change. The feature is internal to the render and app
crates and is removed by reverting the branch.

## Open Questions

- The backdrop budget is a measured constant. The 4000²×8 profiles (fit / 100%)
  measured a Curves tick 670/680 ms → 9.7/8.8 ms and a canvas opacity tick
  150/148 ms → 10.6/8.9 ms, so the default 64 MB is retained; raise it only if a
  profile shows the backdrop being refused on a document the user previews.
