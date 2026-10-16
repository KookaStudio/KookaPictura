# Design: iris-blur-gallery

## Context

Field Blur (`field-blur-gallery`) shipped the Blur Gallery workspace, a
box-pyramid variable blur driven by a per-pixel radius, and the Blur Effects
/ Noise passes gated by `radius / max`. Iris Blur differs only in how the
radius field is built and in the on-canvas controls.

## Decisions

### Share the variable blur, not a trait

`field.rs` exposes `pub(super) fn variable_blur(buf, max_radius, bokeh,
noise, radius_at: impl Fn(x, y) -> f64)`; `field_blur` and `iris_blur` pass
closures. A closure (not a trait) keeps one implementation per caller
(rule 3). Pixel centres are passed in buffer pixels, so Field Blur's
`(x + 0.5) / w` arithmetic is unchanged and its digest holds.

### The iris shape

A pin's outline is the superellipse `(|u/a|^p + |v/b|^p)^(1/p) = 1` in the
frame rotated by `angle` (degrees, clockwise in y-down image space). Both
semi-axes are fractions of the buffer **width** so rotation is rigid on
non-square images. `p = 2 / (1 − 0.009·squareness)` runs 2 → 20; the canvas
diamond sits at the outline's diagonal corner `2^(−1/p)` and inverts the
same map. `feather` is the sharp core as a fraction of the outline
(`ρ ≤ feather` sharp, `ρ ≥ 1` full, smoothstep between). Defaults follow the
owner's screenshot: semi-axes 0.35 of the width (the dialog scales the
vertical one by the picture's aspect), feather 0.65, squareness 50, blur
15 px.

### Several pins

`radius(p) = Π falloffᵢ(p) · Σ wᵢ·blurᵢ / Σ wᵢ` with inverse-square-distance
weights from the pin centres. The product keeps every island sharp; the
blend lets each pin's amount dominate near it and stays continuous as an
amount goes to 0. Inferred — Adobe's combine is closed.

### Slots

`[count, (x, y, rx, ry, angle, squareness, feather, blur)×count, tail(13)]`.
The gallery parser in `filter_map/gallery.rs` is generic over the record
stride and shared with Field Blur (whose canonical all-defaults list now
reads its tail after the pin record rather than overlapping it; the zeroed
canonical list maps identically).

### Workspace

`BlurGalleryDialog` reads `iris_` from the command kind. The Blur Tools rows
are one table; the active row owns the Blur control (`irisBlurSpin` /
`fieldBlurSpin`), the rest are disabled. `BlurGalleryCanvas::setIris` draws
the selected pin's outline and handles; radius handles set a semi-axis and
the angle from the pointer, feather dots set the core along their axis, the
diamond sets squareness.

## Risks

- The falloff and combine are inferred; reference fitting against CS6 output
  is an open question in the doc.
