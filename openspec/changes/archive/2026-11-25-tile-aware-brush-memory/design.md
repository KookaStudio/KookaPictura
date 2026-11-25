# Design

## Context

See `proposal.md` for the motivation and
`specs/document/brush-stroke-memory/spec.md` for the contract. The owner chose
two points on #1603: the undo copy keeps plain boxed tiles, not a tiled `Plane`,
and the gate charges held bytes plus a canvas term.

## Decisions

### The undo copy is a grid of boxed 64² tiles

`Saved` keeps `Vec<Option<Box<SavedTile>>>` over the 64 × 64 grid tiles that
cross the target layer's bounds. A tile is boxed the first time `save` reaches
it, so `None` replaces the old `tiles: Vec<bool>` bitmap. A `SavedTile` holds
only the grid tile's part inside the layer at that moment, with its document
origin and size: one byte block per 8-bit plane the stroke edits and, for a
16-bit tone stroke, one `u16` block per native plane. The block is exactly what
`read_rect_into` returns, so no copy is reshaped.

Clipping matters on an empty layer that grows with the stroke (M6). A first cut
that boxed whole 64² tiles raised M6's peak from 0.79 MiB to 1.0 MiB, because a
75 px stroke spans two or three tile rows. Clipped tiles bring it to 0.54 MiB.

A tiled `Plane<u8>` was the alternative. Its tiles are known to the governor and
can spill, but every saved-pixel read in the dab loop would then go through the
plane's tile lookup. That loop is the hot path the #1238 stroke-speed budget
measures. Boxed tiles keep the read to one index computation and two slice
reads. The heap bytes the governor does not track are bounded by what the dabs
reached, and the gate charges for them (below). Today's dense buffer is not
tracked either.

### The grid is anchored to document coordinates

Tile `(tx, ty)` covers document pixels `[tx·64, tx·64 + 64) × [ty·64, ty·64 + 64)`.
The grid spans the tile columns and rows the layer's bounds cross. When the
layer grows (#1208), the grid is rebuilt for the new bounds and the boxes are
moved into their new slots: no pixel is copied and no tile is allocated. Today's
`grow` allocates a new dense buffer and copies every unsaved pixel of the old
layer into it, so the whole layer becomes resident on the first grow.

A tile saved before the grow holds only the pixels then inside the layer. A
read of the rest of its grid tile returns zero, the correct pre-stroke value of
a grown pixel, since the grown area is new and transparent. A tile not yet saved is read
from the layer when a dab first reaches it, and a dab always saves its box
before it writes, so the grown pixels it reads there are still unpainted.

### The saved-pixel read takes coordinates

`Stencil::composite` takes the pixel's layer-local `(x, y)` instead of the flat
index `i`, so the read is a tile lookup rather than a dense slice index. `base`
becomes a `&Saved`, read through `Saved::pixel(x, y)`, which returns the four planes from one tile lookup. The clip and source
lookups already divide `i` back into coordinates; they now use `(x, y)`
directly.

### The gate charges held bytes plus one byte per canvas pixel

`brush_bytes = Σ allocated_bytes(layer planes) + canvas W × H`.

- **Held bytes.** These bound the reached tiles of a layer's non-empty pixels,
  and they make a dense layer pay its full size, so a stroke that genuinely
  needs the room is still refused. Uniform tiles hold nothing.
- **1 × canvas.** This covers the stroke's own tiles over pixels the layer does
  not hold: the 4 bytes a pixel of saved tiles, the scratch and applied coverage
  (1 byte each), and the history delta at commit, for a stroke reaching about a
  sixth of the canvas on an empty layer. The measured peaks at 4250² sit far
  below this term: M6 0.54 MiB and the dense-layer stroke 2.5 MiB, against
  17.2 MiB and 86.1 MiB of estimate.

In the reproduction (a 21,000² canvas, a sparse 15,000 × 17,500 layer) the old
charge was about 1.0 GB + 1.3 GB. The new one is the layer's held tiles plus
about 0.44 GB, within the 1.9 GB room.

## Risks

- **Stroke speed.** The per-pixel read gains a tile index. The textured Art
  History motion check (#1238) and the stroke benches still run unchanged.
- **Ungated growth.** A long stroke can still allocate tiles past its start
  estimate, as the dense copy's faulted pages could before. Re-gating mid-stroke
  is out of scope.
