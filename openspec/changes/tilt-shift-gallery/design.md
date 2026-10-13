# Design: tilt-shift-gallery

## Context

Iris Blur (`iris-blur-gallery`) builds a per-pixel radius from pin shapes
and hands it to Field Blur's box pyramid and effects passes. Tilt-Shift
differs in the shape (a band, not an outline), in a distortion pass, and in
the on-canvas controls.

## Decisions

### The band

A pin's band runs through its centre at `angle` (degrees, clockwise in
y-down image space). The signed distance across it is
`v = (y − cy)·cos − (x − cx)·sin` (positive below). Each side has its own
focus extent (centre line to the solid line) and fade extent (solid line to
the dashed line), all in buffer **heights** so the default horizontal band
reads naturally; one unit for every extent keeps rotation rigid. Falloff:
0 for `|v| ≤ focus`, 1 for `|v| ≥ focus + fade`, smoothstep between (a step
when fade is 0). Defaults (focus 0.1, fade 0.15, blur 15) are read off the
owner's screenshots and are unverified.

### Several pins

As Iris: `radius = Π falloffᵢ · Σ wᵢ·blurᵢ / Σ wᵢ` with inverse-square
weights from the pin centres.

### Distortion

CS6's slider runs −100..100 % and acts on the bottom blur zone; Symmetric
Distortion acts on both. Modelled as a smear vector per pixel: the
inverse-square blend of each pin's unit direction (radial from the centre
for positive, tangential for negative) times `|distortion|/100`, gated to
the pin's lower side unless symmetric, scaled by twice the local radius.
`smear` averages up to 33 bilinear taps along the vector, centred on the
pixel, reading the pyramid output. A sub-pixel vector (the sharp band) is
untouched. It runs between `pyramid_blur` and `apply_effects`, so
`variable_blur` is split into those two with no byte change (the Field and
Iris digests hold).

### Slots

`[count, (x, y, angle, focus above, focus below, fade above, fade below,
distortion, symmetric, blur)×count, tail(13)]`. Blur stays last in the
record so the dialog's proxy scaling (every stride's last slot) is shared
across the three modes.

### Workspace

`BlurGalleryCanvas` takes a `BlurGalleryMode { Field, Iris, TiltShift }`
instead of the `iris` flag. In Tilt-Shift mode the selected pin's four lines
span the picture (clipped); the rotation dots sit on the solid lines at the
foot of the perpendicular from the centre and are drawn after the amount
label. A press near a line grabs it; past a coincident solid line it grabs
the dashed one so a zero fade can be pulled open. Dragging a solid line
keeps its dashed line in place until it is pushed.

### Hover cursors and the drag draft

The canvas tracks the mouse; the line/dot hit test (`hitTilt`) is shared by
hover and press. Lines show the Qt resize cursor nearest the band's normal
(four shapes, 45° apart); dots show the app's `rotateCursor`. A full proxy
render at 1100 px and Blur 176 measured ~213 ms (optimised), so a pin drag
ran at ~4 fps; the dialog now renders a ≤ 360 px draft (~26 ms) while the
canvas reports `dragging()` or the Blur/Distortion slider is held, and the
full proxy on `dragFinished` / `sliderReleased`. The canvas fits the picture
to the document's aspect so the two images overlay identically.

### Active layer only, edges feather

The workspace first previewed `filter_thumbnail` over the flattened
document, so lower layers blurred with the active one although OK filters
only the active layer (the owner's report). The preview now composes
`filter_proxy_layer` (the active layer, filtered over its own bounds at
proxy scale, mask/opacity baked in) over `filter_proxy_base` (the document
with that layer hidden), at the full or draft edge — the shared filter
dialog's pane path — so the preview is the commit. Separately, the commit
kept a layer's edge hard: the render layer folds alpha in as a fourth plane
for moving kernels (`filters_alpha`), but the gallery pyramid stopped at
three planes (left over from the earlier alpha-copy pass). `pyramid_blur`
and `smear` now run over up to four planes; the effects stay colour-only.
The Field, Iris, and Tilt-Shift digests change for that reason (their
digest picture has four planes; only alpha bytes moved).

### Combined effects

The owner asked for every Blur Tools row to be selectable and expandable, as
in Photoshop, where the effects switched on blur together. The dialog keeps
one `Effect` record per implemented effect (pins, selected pin, on/off, row
widgets); the canvas edits the selected effect's list (`setPins`). OK
commits the single effect's own kind when one is on, so existing history
labels, Last Filter, and slot layouts are unchanged, and the new
`blur-gallery` kind only when several are. The engine's `blur_gallery`
takes the max of the per-effect radius fields (the Field closure, Iris's
`IrisField`, Tilt's `TiltField`, factored out with no byte change); max keeps
each effect's sharp area sharp only where no other effect blurs it, which
matches the owner's expectation of "blur where any blur is" and is marked
inferred. `LightRangeSlider` moved to its own header (pure move) to keep the
dialog under the size cap.

### Empty target

`filter_target_empty` checks the active pixel layer for any non-zero alpha
(or any pixel when it has no transparency channel) inside the selection.
`refuseEmptyBlur` in `frame_menus_filter.cpp` runs it for the Blur and Blur
Gallery families before any dialog, on the menu, Last Filter, and Last
Filter Settings paths: "Could not complete the `<command>` command because
the selected area is empty." for Blur and "…because the initial bounding
rectangle is empty." for the gallery (the owner's screenshot).

## Risks

- Falloff, combine, and distortion are inferred; reference fitting against
  CS6 output is an open question in the doc.
