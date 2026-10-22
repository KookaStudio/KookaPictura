# Design

## Context

See `proposal.md` — Why — for the motivation.

The canvas offset (`ImageView::offset_`) is the scaled document's top-left in
widget space. The paint translates by it (`image_view.cpp:594`), a pan adds the
pointer's delta to it (`image_view_document.cpp:246`), the rulers read it and
the Navigator derives its proxy rectangle from it. It is the single source of
truth, and nothing in this change may alter its meaning.

`offsetRangeFor` (`crates/pictura-app/cpp/canvas_range.h:31`) gives the canvas
the range it may be panned over, `[minX, maxX] = [visibleX - w, V - visibleX]`,
where `w` is the scaled document, `V` the viewport and
`visibleX = min(96, w)` the reveal margin. That range is the document's *flush*
range — the offsets at which the document sits flush against the viewport's
edge — widened by the margin on both sides, so a document can be pulled partly
off the workspace and one display inch of it stays reachable.

The workspace scrollbars project that same range, so their handle can reach
every position the canvas can be panned to. What was wrong is the direction of
the projection: the value counted from the range's minimum, so raising the
handle raised the offset and the content travelled with the handle.

With a 96 px margin, an 800 px viewport and a 1000-px document at 100 %:

```
  ox = offset.x        -904                -200         0                  704
                           |                   |          |                   |
  +---------------------+-------------------+----------+-------------------+
  |    margin 704       |    flush 200      | margin 704                   |
  |                     |                   |          |                   |
  pan extreme        flush-right      flush-left                    pan extreme
                    (document end)  (document start)
```

The bar's track is the whole 1608-px pan range; the value is
`maxX - offset.x`, so it is 0 at the pan's right extreme and 1608 at its left
extreme.

## Goals / Non-Goals

**Goals:**

- The handle travels against the content, on every axis, for every document
  size — including a document that fits the viewport.
- The bar's ends are the document's ends: the start at the bar's minimum, the
  end at its maximum.
- The handle reaches every pannable position, the reveal margin included, and
  rests at the end of its track only when the canvas has reached the pannable
  limit.
- The canvas offset keeps its meaning; the pan, the zoom, the rulers and the
  Navigator are untouched.

**Non-Goals:**

- The reveal margin itself. The pan clamp keeps it, and the bars keep
  projecting it: the margin is a place the canvas can be dragged to, so the
  scrollbar has to be able to say so.
- The pan policy: Free Pan and the workspace snap lock keep their rules and
  their collapses.
- The Navigator's proxy, the rulers, the zoom ladder and the fit commands.

## Decisions

### The range stays the shared pan range

The bars keep deriving their range from `offsetRangeFor`, the helper that
clamps the canvas pan, exactly as they did before. The bar's range is therefore
the set of offsets the canvas can be panned to, and the value is a bijection
onto it: every pannable position has a handle position, and the handle reaches
the end of its track only when the canvas is at the pan's limit.

*Alternative:* give the bars the document's *flush* range — the offsets at
which the document sits flush against the viewport's edge — so the bar's ends
are the document's flush edges and the handle's travel matches the document's
travel exactly. This was implemented and rejected: the reveal margin lies
outside the flush range, so the canvas could be panned into the margin while
the handle sat at the end of its track, and the bar could not express where in
the margin the canvas was. The handle resting at its end while the canvas can
still move is the defect this change is about, so a range that cannot express
the margin cannot be the answer.

### The value counts from the range's maximum

    value  v  in [0, maxX - minX]        ox = maxX - v        v = maxX - ox

The range's maximum offset is the pan that shows the start of the document (its
leading edge at the viewport's leading edge), so the bar's minimum shows the
start and its maximum the end, and raising the value lowers the offset, so the
content travels against the handle.

For a document that fits the viewport the range is the placement freedom the
margin leaves around the centred document, and the same formula puts the bar's
minimum at the document parked against the viewport's trailing edge. The
alternative anchor — the minimum at the leading edge, so "0 %" always means
"the document's leading edge at the workspace's left" — would make the document
slide *with* the handle on a fitting document, which is the very pattern the
issue reports as a bug, and a fitting document is the common case. The
direction wins; the anchor is meaningless for a document with no content order.

## Risks / Trade-offs

- [The bar's ends park the canvas partly off the workspace] → the bar's ends
  are the pan range's ends, so the handle's extremes are the same positions the
  hand tool can reach: the document's start or end, partly off the workspace,
  with the reveal margin visible. That is the pan's own limit, and the bar now
  says so instead of hiding it.
- [The margin takes most of the handle's travel when the document barely
  overflows the viewport] → a 1000-px document in an 800-px viewport moves over
  200 of the 1608-px track. The margin is a real place the canvas can be
  dragged to, so the bar shows it; the alternative — a track of the document's
  travel alone — cannot show it at all.
- [The handle's rendered size is diluted by the margin] → `QScrollBar` sizes
  the handle at `pageStep / (span + pageStep)` of the track, and the span
  carries the margin, so the handle is smaller than the visible fraction of the
  document. That is the behaviour the bars had before this change; nothing here
  makes it worse.
- [An empty document (0 × 0) reads the bar's maximum instead of its minimum] →
  `clampOffset` pins an empty document's offset to `(0, 0)` rather than
  clamping it into a range, so the value lands on the far end. Nothing
  navigable is involved, and the document cannot be created empty from the UI.
- [The self-test's `canvas_scrollbar_roundtrip` check asserts the old
  projection] → it is updated in place, keeping its exit code 309; no
  `ST_BEGIN` site is added, so the self-test budget does not move.
