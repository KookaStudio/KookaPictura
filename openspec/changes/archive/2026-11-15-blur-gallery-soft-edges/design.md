# Design

## The rim

The blur field returns a per-pixel weight (`radius / max_radius`). Light Bokeh
multiplied the *blurred* pixel by `1 + 2·lift`, with `lift` proportional to the
blurred luma. On a dark subject against a bright background the blurred edge has
a higher luma than the subject's interior, so the edge is lifted more than the
interior. The hard `low ≤ luma ≤ high` test then switches the lift off on a
contour, which turns the over-lift into a visible bright rim.

## Apply the lift to the source

`pyramid_blur` now calls `apply_bokeh` after it computes the field weights and
before it clones the source planes for the pyramid. The lift and tint therefore
act on the sharp source pixels that fall inside the Light Range, and the blur
mixes the lit and unlit source. A pixel with weight 0 is still untouched, so the
"sharp pixels never change" invariant holds. Along a step, the output is a
weighted mix of the lit dark region and the unlit bright region: its ratio to
the unlit blur falls monotonically from the interior ratio toward 1, so it can
never exceed the interior ratio.

The Noise pass needs the blurred pixels, so it stays after the pyramid; it is
split into `apply_noise` and called with the same weights.

## Soft Light Range

`light_range_gate` is a product of two smoothsteps with a feather of a quarter
of the range width (at least one level). It is 0 at both bounds and 1 across the
middle, so the lift and tint taper out instead of switching off. The effect is
unchanged for a tonal plateau well inside the range, which keeps the existing
Light Bokeh and Bokeh Color tests valid.

## Digests

The Field, Iris, Tilt-Shift, and combined gallery kernel digests change because
the bokeh now feeds the blur. They are updated together, with the render-order
change called out in the commit.
