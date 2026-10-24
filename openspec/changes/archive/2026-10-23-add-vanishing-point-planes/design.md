# Design: add-vanishing-point-planes

Adobe's plane solver is closed. What follows is projective geometry, chosen
so that CS6's documented behaviour falls out of it. It is an approximation,
not verified parity.

## Camera

- **Principal point and units.** The principal point is the document centre.
  Image coordinates are measured in half-diagonals from it.
- **Vanishing points.** A root quad's edge pairs 0→1 / 3→2 and 0→3 / 1→2
  meet at the vanishing points `v₁`, `v₂`, in homogeneous coordinates.
- **Focal length.** A rectangle's edge directions are perpendicular only
  when `f² = −(x₁x₂ + y₁y₂) / (w₁w₂)`. When `f² ≤ 0`, no camera with that
  principal point sees the quad as a rectangle, and the plane is yellow.
- **Parallel edges.** A vanishing point more than 20 half-diagonals out
  counts as at infinity, because a parallel pair leaves `f` free. A normal
  lens (`f = 2.3`, about a 47° diagonal angle of view) is then assumed.
- **Red quads.** A crossed, concave or degenerate quad (under 4 px²) has no
  aspect ratio and is red.

## Lifting a plane to 3D

- **The plane.** `H` maps the unit square to the normalised corners, so
  `K⁻¹·H = [U V O]` gives the plane's edge vectors and its corner 0.
- **Scale.** The plane is scaled so it lies at unit distance in front of the
  camera. Its axes are `U/|U|` and `V/|V|`.
- **Metric.** One metric unit's 3D length is set so the plane's metric area
  equals its image area in px². A plane seen square-on therefore measures
  its pixel size, and Grid Size reads roughly in pixels.
- **When the metric is kept.** Resizing an edge keeps the plane and its unit
  and moves only the rect, so grid lines stay put on the surface. A corner
  drag changes the plane itself and re-fits the unit.

## Related planes

- **Storage.** A related plane stores its parent, the parent edge, an angle
  `θ`, a side (the sign of the parent normal), and a span `[f0, f1, t0, t1]`.
  - `f` is a fraction of the parent's edge, so a related plane stays on the
    whole shared edge when its parent is reshaped. A span it was given
    explicitly is kept as the same fractions.
  - Its frame's `s` axis runs along the edge in the parent's own
    coordinate, so grid lines continue across the hinge.
  - `t` runs away from the edge along `D = cos θ·P + sin θ·side·N`.
  - `P` is the outward in-plane perpendicular, `N` the parent normal.
  - `θ = 0` lies flat beyond the edge, 90 is perpendicular, 180 folds back.
- **Tear-off.** Ctrl-drag on an edge node creates the plane at 90° spanning
  the edge. Its far edge follows the pointer, on the side of the parent the
  pointer is on.
- **Turning.** Alt-drag on the far edge node, or the Angle box, sets `θ`.
  Only a related plane with no children of its own may turn.
- **Inheritance.** A family shares its root's camera, metric unit and Grid
  Size. An invalid root makes the whole family take its colour, and each
  related plane then shows its last good outline.
- **Moving.** Moving a related plane slides it along its hinge.

## Editing

- **Hit test.** Nodes win: first the active plane's, then the newest
  plane's, within 5 screen pixels. Then the plane under the pointer.
- **Gestures.** A root corner moves freely. Edge nodes resize in
  perspective. An invalid root's edge simply follows the pointer. Dragging
  the inside moves a root in the image.
- **Delete.** Deleting a plane deletes the planes hinged on it *(inferred)*.
- **Undo.** The dialog's undo holds 100 plane-set states. A whole drag is
  one state.

## Render Grids and the commit

- **The edit.** OK with Render Grids on turns every valid plane's outline
  and grid into line segments in the active layer's pixels. That `VpEdit` is
  kept by id; it is the `vanishing-point` slot.
- **The kernel.** The kernel draws the lines one pixel wide, antialiased,
  in black. Coverage is the maximum over segments, so crossings do not
  darken twice.
- **Why the filter path.** Because the commit runs through the ordinary
  filter path, the selection clips it, Fade applies, and the state is named
  "Vanishing Point".
- **Without Render Grids.** OK without Render Grids keeps the planes on the
  document and records no state. Cancel keeps nothing.

## Ceilings (`ponytail:`)

- The rendered grid's colour and weight, and the Grid Size range (1–1000),
  are not confirmed against CS6.
- Kept edits stay in memory for the process (eight of them). The planes are
  not written to files.
- Grids denser than 1000 lines per axis show every n-th line.
