# Design: path-blur-gallery

## Context

Field Blur, Iris Blur, and Tilt-Shift each build a per-pixel **radius** and
share one isotropic box pyramid. Path Blur is directional: it needs a
per-pixel **motion vector** and a gather along it, so it is a new pass, not
a new radius field. Sources: the owner's CC screenshots (the opening
workspace, the Path Blur row, the Motion Effects tab, Edit Blur Shapes),
the 2014 Adobe Help snapshot and CreativePro (secondary) recorded in
`docs/06-filters/blur-gallery.md`. Everything numeric below is inferred
unless it names a screenshot.

## Decisions

### Paths

A path is 2..=`MAX_PATH_POINTS` (32) points, fractions of the buffer
(outside allowed). The drawn curve is the uniform Catmull-Rom spline
through them (end tangents mirrored), sampled at 16 segments per span; the
owner's screenshots show smooth curves through the points. Each end carries
an **End Point Speed** in buffer pixels (`0..=MAX_END_POINT_SPEED`, 500;
unverified) — the length of that end's blur shape. Speeds along the curve
interpolate linearly by arc length.

### Motion field

For a pixel, each path contributes its nearest curve point's unit tangent
`T`, the interpolated speed `s` there, and the inverse-square weight
`1 / ((d / width)² + ε)` of its distance. The motion vector is
`Σ w·T·s / Σ w` times `speed / 100`. One path therefore moves the whole
picture (the opening screenshot blurs everything along its one short path);
several blend as Field Blur's pins do. The field is evaluated on a grid of
cells (`ceil(long edge / 512)` px, at least 1) and bilinearly interpolated,
so the gather costs no path search per tap.

### The gather

A pixel's trail length is `L = |motion|` at the pixel; below one pixel it is
untouched. Taps `n = clamp(ceil(L) + 1, 2, 64)` are placed by walking the
**streamline** (re-reading the field's direction at each step, so curved
paths swirl) `L / (n − 1)` apart: **Centered Blur** (default on, screenshot)
splits the walk half behind and half ahead of the pixel; off, the whole
trail lies behind it (a moving subject smears forward). Weights:

- **Taper** (`0..=100 %`): `1 − taper·u`, `u` the tap's distance from the
  pixel over the half-trail (centred) or trail, so the far ends fade.
- **Strobe** (`Strobe Strength 0..=100 %`, `Strobe Flashes 1..=100`,
  default 0 % / 4 from the screenshot): flashes spaced evenly over the
  trail, each landing on its nearest tap with that tap's taper weight; the
  continuous and strobe weights are each normalized and mixed by strength.
- **Rear Sync Flash**: the pixel's own tap gains the whole trail's weight
  again (the subject frozen sharp at the end of the exposure).

The gather runs over all planes up to four, alpha included (layer edges
smear into transparency, as the other gallery blurs feather). Then the
Noise pass, weighted by `L / max L`. ponytail: Adobe's motion field and
flash model are closed; the inverse-square blend, the 64-tap streamline,
and the weights above are inferred.

### Presets

The row's dropdown is **Basic Blur** / **Rear Sync Flash** (screenshot);
it reads **Custom** while Strobe Strength is above 0 (screenshots), and
choosing a preset from Custom zeroes the strength. The engine sees only
`rear_sync` and the strobe values.

### Combining

`blur_gallery` with paths: the pyramid (if any radius effect is on), the
Tilt-Shift smear, then the path pass over that result, then the effects.
Bokeh (Effects tab) uses the radius weights only — CC lists it with the
three radius effects (inferred); Noise uses the per-pixel larger of the
radius and path weights. When both a radius effect and paths are on, the
bokeh and noise passes run separately; without paths the single combined
call is unchanged, so every existing digest holds.

### Slots

`path-blur`: `[count, (point count, (x, y)×point count, start speed, end
speed)×count, speed, taper, centered, rear sync, strobe strength, strobe
flashes, tail(13)]`. The canonical list is one two-point path (27 slots).
`blur-gallery` appends the path count, records, and the six motion slots
after the Tilt-Shift section (51 canonical); its all-defaults form now also
requires the path count 0. The dialog scales the end speeds by the proxy
factor as it does every blur amount.

### Workspace

`BlurGalleryMode::Path`. The dialog keeps one `BlurGalleryPin` per path
(new `points` and `speeds[2]` members); the motion values are dialog-level,
shared by every path. The opening path starts at the picture's centre and
runs right 0.08 of the width, with both end speeds 0.04 of the document
width (the owner's opening screenshot at 100 %: 67 px long, ~33 px blur
shapes, on an ~830 px picture).

Canvas gestures. Placing a path follows the owner's CC walkthrough; the
rest are inferred:

- click on empty picture: pin a new path's first stop (copying the selected
  path's end speeds); its last point then floats with the pointer, each
  click pinning another stop, so the curve through the stops previews live
  (as a draft, like a drag). A double-click, Enter, Esc, or a click on the
  last stop ends it, dropping the floating point; ended on its first stop
  it runs right the opening length. Qt delivers a double-click's second
  press as the double-click event, so the first click pins the last stop
  and the double-click only ends the path (and never starts one);
- press a point and drag: move it (selects the path and the point);
- press the curve between points: insert a point there and drag it;
- Ctrl-press a path: move the whole path;
- Delete: remove the selected point when its path keeps two, else the
  selected path when another remains.

Hovering empty picture or a curve shows `cursor.pinAdd` (a hand-drawn
pushpin with a plus); a point shows the arrow. OK while a path is being
placed keeps its pinned stops.

### Tabs and Noise

Bokeh does not apply to paths and the Motion Effects apply only to them, so
selecting Path Blur disables the Effects tab and enables Motion Effects, and
selecting another effect does the reverse (the owner's screenshots). The
Noise checkbox defaults on as in CC — `BlurNoise::default()` and the
mapping's default too — which changes no output, since Amount defaults
to 0.

End Point Speed is enabled only while an end point is selected and edits
that end. Edit Blur Shapes is shown disabled (ponytail: the per-end blur
shape's direction is the path tangent; only its length is editable). The
selected path draws brighter with its end arrow; an end point's selected
state is a filled dot.

### Files

`blur_gallery_dialog.cpp` (907 LOC) and `blur_gallery_canvas.cpp` (632)
cannot absorb the Path code under the 800-line target, so the Path row,
Motion tab, and path slot reading live in `blur_gallery_path.cpp`, and the
path hit-testing, drags, and painting in `blur_gallery_canvas_path.cpp`
(members of the same classes; no new abstraction).

## Risks

- A 6000 px commit with long trails is ~64 bilinear taps × 4 planes per
  pixel; parallel, a few seconds. The draft preview covers live drags.
- The motion field cancels where opposing paths meet (as averaging
  vectors does); Adobe may resolve that differently.
