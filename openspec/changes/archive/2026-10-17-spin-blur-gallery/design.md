# Design: spin-blur-gallery

## Context

Path Blur added a directional gather after the radius-field pyramid. Spin
Blur is a second directional pass: a per-pixel **arc** around a rotation
centre instead of a streamline. Sources: the owner's CC screenshots in
issue #464 (the Spin Blur row with Blur Angle 15°, 30°, 87°, 180°; the
Motion Effects tab with Strobe Strength, Strobe Flashes, Strobe Flash
Duration; circular and elliptical spins with handles; two spins at once),
the 2014 Adobe Help snapshot and PCWorld (secondary) recorded in
`docs/06-filters/blur-gallery.md`. The owner also attached a chatbot
summary of the algorithm; it is a hint, not a source. Everything numeric
below is inferred unless it names a screenshot.

## Decisions

### Spins

A spin is an ellipse: centre `x, y` (fractions of the buffer, outside
allowed), semi-axes `radius_x, radius_y` as fractions of the buffer
**width** (one unit, as Iris, so rotation keeps the shape), `angle` turning
the `radius_x` axis clockwise in degrees, and `feather` (0..=1) the inner
ring as a fraction of the outline. `blur_angle` (0..=`MAX_SPIN_ANGLE`, 360°)
is the arc's sweep. `pivot_x, pivot_y` offset the rotation centre from the
ellipse centre (fractions of the buffer; 0 by default), so Alt-drag moves
the centre the picture spins round while the ellipse stays put.

### The mask

`rho` is the pixel's normalized elliptical radius. The mask is 1 inside
the feather ring (`rho ≤ feather`), 0 outside the outline (`rho ≥ 1`), and
a smoothstep fall between, the reverse of Iris's falloff (the screenshots
blur inside the ellipse and leave the picture sharp outside). A pixel in
several spins takes the spin with the largest mask, the first on a tie.

### The gather

Inside the feather ring a pixel sweeps the full Blur Angle `θ` round the
pivot, arc length `L = r·θ` (`r` the distance to the pivot). Past the ring
the sweep is `θ · mask · feather / rho`, so `L` is the ring's arc length
times the mask: the smear only shortens toward the outline. (Fading the
angle alone lets `r` outgrow the mask, so the blur would peak mid-band and
reach the outline; the owner's CC comparison shows it fading there.) Below
one pixel it is untouched.
Taps `n = clamp(ceil(L) + 1, 2, 128)` are evenly spaced in angle over
`[−θ/2, θ/2]`, centred on the pixel (the screenshots show no direction),
read bilinearly from the source. Weights are uniform; with strobe:

- **Strobe Strength** (`0..=100 %`), **Strobe Flashes** (`1..=100`),
  **Strobe Flash Duration** (`0..=360°`; default 0 % / 4 / 1° from the
  screenshots): the flashes are evenly spaced over the sweep end to end (a
  single flash sits on the pixel), each lighting the taps within half its
  duration of it (at least the nearest tap). The continuous and strobe weights are each
  normalized and mixed by strength, as Path Blur mixes its flashes.

The gather runs over all planes up to four, alpha included. Then the Noise
pass, weighted by the mask. ponytail: Adobe's mask and flash model are
closed; the smoothstep, the largest-mask combine, the 128-tap arc, and the
flash windows are inferred. Long arcs (far from the pivot at large angles)
undersample to 128 taps and can ghost.

### Combining

`blur_gallery` with spins: the pyramid and Tilt-Shift smear, the path
pass, then the spin pass over that result, then the effects. Bokeh stays
gated by the radius weights; Noise uses the per-pixel largest of the
radius, path, and spin weights. With no spins nothing changes, so every
existing digest holds.

### Shared strobe

CC shows one Motion Effects tab: with Path Blur selected it lists Strobe
Strength and Strobe Flashes, with Spin Blur selected Strobe Flash Duration
too (the screenshots). The dialog keeps one strength and one flash count for
both effects (inferred) and shows the duration only while Spin Blur is
selected. The engine stays decoupled: `Filter::SpinBlur` carries a full
`SpinStrobe`, and the `blur-gallery` mapping builds the spin's strobe from
the motion section's strength and flashes plus the one duration slot.

### Slots

`spin-blur`: `[count, (x, y, radius x, radius y, angle, feather, blur
angle, pivot x, pivot y)×count, strobe strength, strobe flashes, flash
duration, tail(13)]`; canonical one spin (26 slots). `blur-gallery`
appends the spin count and records after the path records, then the six
motion slots, then the flash duration (62 canonical); its all-defaults form
requires the spin count 0 too. The Blur Angle is not scaled by the proxy
factor: an angle is the same at any resolution.

### Workspace

`BlurGalleryMode::Spin`. A spin is a `BlurGalleryPin` with `radii`,
`angle`, `feather`, `blur` (the Blur Angle), and a new `pivot` offset. The
opening spin is a circle centred on the picture, radius 0.13 of its width,
feather 0.5, Blur Angle 15° (the owner's opening screenshot: a 125 px
radius on a 963 px picture at 15°; the feather is unverified).

Canvas: the selected spin draws Iris's outline (an ellipse, no roundness
diamond) with four radius handles (drag to resize and turn) and four
feather dots. Each spin's marker sits at its rotation centre with the
gallery pin ring, whose arc shows the Blur Angle (full at 360°); dragging
the ring changes it (1° per pixel), dragging the marker moves the spin,
Alt-dragging it moves only the rotation centre. A click on empty picture
adds a spin copying the selected one's outline and angle (rotation centre
reset). Delete removes the selected spin while another remains.

The Spin Blur row shows Blur Angle (`0..=360°`). Selecting Spin Blur
disables the Effects (Bokeh) tab and enables Motion Effects with the
duration slider shown, as Path Blur does without the duration.

### Files

`blur_gallery_dialog.cpp` (971 LOC) cannot take the Spin row, records,
and strobe reading; they live in `blur_gallery_spin.cpp` (members of the
same class). The canvas changes are small (a mode check on the Iris
handles, the marker at the pivot, the ring's scale, the Alt-drag) and stay
in `blur_gallery_canvas.cpp`.

## Risks

- A 6000 px commit with large angles is up to 128 bilinear taps × 4 planes
  per pixel inside the ellipses; parallel, a few seconds. The draft preview
  covers live drags.
- The slider curves in the screenshots are non-linear (15° at ~37 % of the
  track); ours are linear.
