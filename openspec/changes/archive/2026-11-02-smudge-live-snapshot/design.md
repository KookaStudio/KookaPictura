# Design

## Context

`SmudgeBrush::dab` lays the patch carried from the previous dab, then reloads
the patch from the pickup surface. Without Sample All Layers the surface is the
working layer `img`, which already holds the smear just laid, so the feedback
loop drags paint along the stroke. With Sample All Layers the surface is
`sampled`, a layer-local composite built at the press; it never changes, so the
loop is broken and the stroke replays press-time pixels.

The stroke engine (`pictura-paint`) has no compositor dependency, and the
reduced-level preview path replays the exact stroke at commit with no presents,
so reading `Document::composite` per dab would not be current on that path.

## Decision

Build each carried patch from `sampled` (the press-time composite) with the live
working layer `img` composited over it, using straight-alpha source-over. The
working layer is the only part of the document a stroke changes; every other
layer is static until the stroke ends.

This keeps the fix inside `pictura-paint`:

- Sample All Layers off is unchanged (`img` is the pickup surface).
- An opaque topmost target is exact: `img` covers the press-time target.
- Smudging onto an empty topmost layer is exact: transparent `img` leaves the
  sampled composite, and the laid pixels appear as the finger paints.
- A partial-alpha target double-counts its press-time contribution under the
  live pixels, and a target below other layers composites its live pixels above
  them. Both are approximations, marked with a `ponytail:` comment.

The alternative — an app-provided live composite — would be exact but touches
the stroke API, the preview replay, and the crate boundary, and still needs a
per-dab compositor call.

## Testing

- Engine: a Sample All Layers stroke on a transparent target over a red/blue
  composite; after dragging, the far end carries the laid red rather than the
  press-time blue.
- Qt Test: with a single opaque layer, Sample All Layers on and off produce the
  same smear reach; before the fix the Sample All Layers stroke stops short.
