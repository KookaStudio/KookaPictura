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
  `(1 − density) + density·(1 − t²)²`. Every tool is "where does the new
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
- **Preview.** `LiquifyProxy` box-reduces the layer by a whole factor to at
  most 3072 px, premultiplied, and renders proxy rectangles with a bilinear
  mesh and bilinear source; frozen areas are tinted at half strength when
  Show Mask is on. A dab re-renders only its rectangle and the C++ canvas
  paints that patch in. *(approximation below 100 %; the commit is exact)*

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
  column. Shortcuts W R C S B O X T F D H Z; Mirror takes X so M can be Load
  Last Mesh (the spec's reading). `[` / `]` resize the brush by 10 %. Alt
  reverses a stroke; Shift-click draws a straight line from the last click.
  The tool glyphs are drawn with QPainter in the palette's text colour (the
  icon set has none).
