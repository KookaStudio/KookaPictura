# Design: add-liquify

## Mesh

- **Inverse field.** `LiquifyMesh` stores, per node of a regular grid over the
  layer, the offset `d` such that output pixel `p` shows the source at
  `p + d(p)`. An inverse map never leaves holes. Node 0 sits on the first pixel
  centre; the grid is 2 px up to ~4.2 MP, 4 px to 32 MP, 8 px beyond, so a
  PSB-sized layer's mesh stays tens of megabytes (`FILT-090` edge case).
- **Dabs.** Ported from photocraft's `LiquifyField`: a stroke dabs every fifth
  of the brush radius; each node within the radius takes
  `k = pressure × falloff × (1 − freeze)`, falloff
  `(1 − density)·core + density·(1 − t²)²` where `core` is flat to 85 % of
  the radius and smoothsteps to zero at the rim (photocraft's
  `(1 − density) + …` stops at `1 − density` on the rim, a step every dab
  repeats into a jagged ring). Every tool is "where does the new
  output at `p` come from in the current output": Forward Warp `p − kΔ`, Push
  Left `p` plus Δ turned 90° (Alt: the other way), Twirl a rotation by
  `±0.12·k·rate` about the centre, Pucker/Bloat a scaling by `1 ± 0.06·k·rate`.
  The new offset is `q + d(q) − p`, read bilinearly from a snapshot taken
  before the dab, so a dab never reads its own writes. Reconstruct scales `d`
  by `1 − k(0.1 + 0.4·rate)`. *(approximation: strengths tuned by eye)*
- **Mirror** (Kooka's). The side left of the stroke (right with Alt) shows the
  far side reflected across the stroke line, blended by `min(k, 1)`. The far
  side is never written, so overlapping dabs keep reflecting the same pixels.
- **Turbulence** (Kooka's). `q = p + 0.06·k·rate·s·n(p/s)` with `n` a smooth
  vector value noise on a fixed hash and `s = r(1 − 0.85·jitter)` (≥ 2 px):
  higher Turbulent Jitter scrambles finer. Deterministic.
- **Held tools.** Reconstruct, Twirl, Pucker, Bloat, Turbulence, Freeze and
  Thaw dab once on press and once per Brush Rate tick (40 ms) while held.
- **Far-off segments.** Only the steps whose dab can reach a node are walked
  (photocraft #937), so a stroke to a huge coordinate returns at once.
- **Whole-mesh edits.** Reconstruct scales unfrozen offsets by
  `1 − amount·(1 − freeze)` (100 restores exactly); Restore All zeroes every
  offset; Mask All / None / Invert All rewrite the mask. `scaled_to` stretches
  a mesh corner-anchored to another layer size, scaling offsets with it.

## Resampling

- **Commit.** `liquify` interpolates the mesh with Catmull-Rom per pixel and
  samples each colour plane with Catmull-Rom, edges repeating. Rows whose four
  influencing node rows are all zero are skipped, and coordinates within 1e-7
  of a pixel snap, so an identity mesh (or a fully reconstructed one) leaves
  the layer bit-identical. A mesh of another size is stretched to fit.
- **Alpha.** `filter_preserves_opacity` lists Liquify, so the shared
  unlocked-layer pass runs the same warp over the alpha plane.
- **Preview.** `LiquifySource` shares the layer's full-resolution planes and
  renders the dialog's view at screen (device) resolution: magnified, each
  screen pixel samples the commit's Catmull-Rom mesh and Catmull-Rom source,
  so a zoomed-in preview is smooth and matches OK; between 50 % and 100 % it
  is bilinear; minified past 2:1 it reads a lazily built box-halved level.
  Frozen areas are tinted at half strength when Show Mask is on. A dab
  re-renders only the view pixels over its rectangle; pan and zoom re-render
  the view.

## App

- **Session.** `filter_tools/liquify.rs` keeps one session (the dialog is
  modal) holding the mesh, the proxy, the rendered preview and the brush.
  `liquify_finish(true)` keeps a mesh that moves something in an 8-entry store
  and returns its id; the `liquify` kind's one slot names it, as Displace's map
  does, so `filter_commit_start` runs it on a worker with one history state,
  and Last Filter / Last Filter Settings replay or reload it. Id 0 is an
  identity mesh; an id no longer kept is refused. Load Last Mesh reloads the
  most recent kept id, stretched to the layer, keeping the session's mask.
- **Dialog.** `LiquifyDialog`: tool column (Basic: Forward Warp, Reconstruct,
  Pucker, Bloat, Push Left, Hand, Zoom; Advanced adds Twirl, Mirror,
  Turbulence, Freeze, Thaw), the canvas with a zoom row, and the options
  column. It opens at 100 %, centred; a right click with the Zoom tool (or on
  its button) offers CS6's levels 6–1600 %, Actual Pixels, Fit in View (never
  past 100 %) and Fit on Screen. Shortcuts W R C S B O X T F D H Z; Mirror takes X so M can be Load
  Last Mesh (the spec's reading). `[` / `]` resize the brush by 10 %. Alt
  reverses a stroke; Shift-click draws a straight line from the last click.
  The tool glyphs are drawn with QPainter in the palette's text colour (the
  icon set has none).

## View Options

- **Show Image** hides the distorted layer (the mesh and backdrop stay).
- **Show Mesh** draws the source grid (Small / Medium / Large = 16 / 32 /
  64 layer px; Red … Gray, default Gray) as the distortion places it. Each
  grid line is the contour where the source coordinate `p + d(p)` crosses a
  multiple of the cell, traced by marching squares on a lattice about three
  screen pixels apart over the visible part of the layer. Contours stay
  correct through twirls and folds, where inverting the mesh point by point
  does not converge. *(cell sizes are approximations)*
- **Show Backdrop** reads, once per Use choice, either All Layers (the
  document composite with the active layer hidden, display-converted) or one
  pixel layer, placed in the active layer's frame
  (`liquify_backdrop`). In Front draws it over the distorted layer at
  Opacity, Behind under it; Blend draws it with Difference at Opacity
  (*approximation: CS6's Blend formula is unpublished*). Only the active
  layer is ever distorted. Defaults: All Layers, In Front, 50.
- Leaving Advanced Mode turns Show Mesh, Show Mask and Show Backdrop off and
  Show Image on, as Basic mode has no View Options.
