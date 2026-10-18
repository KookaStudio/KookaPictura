# Design: large-tool-performance

## Measuring

`scripts/profile-tools.py` launches `pictura --headless --control`, opens a
document, and drives each scenario through the control server: `set_tool`,
`tool_options`, then `pointer` `down`, one `move` request per motion event
(40 here, a zigzag across the middle of the document), and `up`, then `undo`.
Every request is timed on the client, so a move's time is the GUI thread's
work for that event plus whatever it deferred (the frame flush, the repaint)
before it read the next request. `settle` is a trailing `status` request: work
deferred past the release. 16 cores, RelWithDebInfo, offscreen, GPU compute on
(the app default).

```bash
python3 scripts/profile-tools.py --size 4000x3000 --save before.json
python3 scripts/profile-tools.py --size 4000x3000 --compare before.json
python3 scripts/profile-tools.py --image worldmap_16000x16000.png --timing-log t.log
```

### 4000×3000 (new white document), ms

| scenario | move p95 | release | total | undo |
| --- | ---: | ---: | ---: | ---: |
| brush ⌀40 | 8.5 → 9.4 | 56 → 9.7 | 299 → 258 | 26 → 31 |
| brush ⌀500 | 26.6 → 10.1 | 92 → 12 | 840 → 297 | 68 → 47 |
| brush ⌀1940 | 209 → 35 | 142 → 24 | 1843 → 378 | 135 → 133 |
| pencil ⌀1940 | 137 → 23 | 85 → 18 | 1100 → 270 | 138 → 137 |
| eraser ⌀1940 | 136 → 33 | 61 → 12 | 1256 → 356 | 106 → 111 |
| rectangle (Shape) | — | 180 → 50 | 208 → 83 | 33 → 25 |
| ellipse (Shape) | — | 450 → 47 | 479 → 78 | 25 → 27 |
| rectangle (Pixels) | — | 220 → 64 | 252 → 93 | 30 → 31 |
| marquee | — | 49 → 52 | 98 → 104 | 6 → 5 |
| gradient | — | 1395 → 258 | 1422 → 288 | 139 → 99 |
| paint bucket (click) | — | — | 905 → 444 | 154 → 86 |

### The world map (16507×16196), ms

| scenario | move p95 | release | total | undo |
| --- | ---: | ---: | ---: | ---: |
| brush ⌀40 | 15 → 17 | 429 → 100 | 985 → 672 | 477 → 458 |
| brush ⌀500 | 87 → 22 | 485 → 124 | 3119 → 865 | 578 → 550 |
| brush ⌀1940 | 304 → 51 | 1096 → 171 | 9890 → 1882 | 846 → 839 |
| pencil ⌀1940 | 169 → 37 | 625 → 155 | 4969 → 1444 | 787 → 785 |
| eraser ⌀1940 | 301 → 46 | 1039 → 153 | 10179 → 1763 | 807 → 780 |
| rectangle (Shape) | — | 2721 → 735 | 2762 → 775 | 577 → 578 |
| ellipse (Shape) | — | 10201 → 740 | 10242 → 783 | 549 → 572 |
| rectangle (Pixels) | — | 5676 → 1483 | 5710 → 1517 | 674 → 665 |
| marquee | — | 1861 → 1920 | 3222 → 3311 | 39 → 41 |
| gradient | — | 32441 → 4850 | 32485 → 4897 | 2607 → 2649 |
| paint bucket (click) | — | — | 7183 → 5483 | 11 → 12 |

## Why it was slow

**The brush tools never took the large-brush paths.** The reduced-level
preview and the GPU stroke (`stroke-lod-preview`, `gpu-stroke-rendering`) hang
off `PictureView::begin_paint`, which only the self-test calls; the shell's
Brush, Pencil and Eraser begin through `paint_tools::begin` and always run the
exact CPU raster. A ⌀1940 dab rastered its 3.7 MP box on one thread in
~200 ms, and with 25% spacing most moves lay a dab, so a move cost 210–310 ms.
The commit then re-composited the whole stroke extent, because in-stroke
presents deliberately did not patch the stored composite: 0.33–0.92 s on the
map.

**Shapes.** `shape_add_layer` re-composited the document. That alone is
0.5 s at 8000² (`shape_commit_profile`), but an ellipse cost 2.5 s there: a
vector mask's row table covers only rows the path crosses, and a sample in any
other row fell through to the uncached path, which walks every edge of the
flattened outline per pixel. A rectangle has four edges; an ellipse, hundreds.

**Fills** (`fills::apply`) re-composited the document although every edit
reports the rectangle it changed, and `fill_layer` walked the whole layer on
one thread.

## Decisions

**D1 — Make the exact raster fast rather than route the tools onto the
preview.** The preview defers the exact raster to the commit (a ⌀1940 stroke
replays every dab at release, ~200 ms each), and the GPU stroke seeds with a
whole-layer upload (1 GB on the map). Rastering the exact dab's rows in
parallel helps every Paint-kind tool, keeps the committed pixels exactly what
they were, and needs no new path. A dab is split into bands of whole rows of
~16 K px when its box is at least 128² px; below that one thread is cheaper.
Each band owns disjoint row slices of the coverage buffers and the four
planes, so no locking; Dissolve keeps one thread because its random rolls
follow raster order. `a_banded_large_dab_matches_the_serial_raster` holds the
bands to a forced-serial raster across modes, tips, dynamics, a lock and a
stamp source.

**D2 — Present into the composite during the stroke.** `refresh_region_buffer`
now patches `doc.composite` for a layer stroke as well, so the commit only
presents the pending frame (`take_pending_present`). Cancel used to rely on
the composite being pre-stroke; it now re-composites the stroke's extent after
restoring the tiles (`cancel_stroke`), which also covers undo during a stroke.
A mask stroke keeps the old rule (its cancel restores from the composite), and
a previewed stroke still refreshes its dirty tiles, since it never presented
full resolution.

**D3 — Answer rows beyond the path at once.** `Rows` records the path's row
span; a row outside it is outside under either fill rule, so `inside` returns
before the edge scan. The table and its clip bound are unchanged.

**D4 — Bounded shape and fill commits, with conservative fallbacks.** A new
shape layer's effect is confined to its outline plus its stroke width (the
stroke is a layer effect, and the outline is padded by its full width
whatever the alignment) plus a pixel; a fill's to the rectangle its edit
returns. Both use `refresh_edit`, which composites that region and emits
`changed` without bumping `frame_revision`. A layer style elsewhere in the
tree could carry the change further (a group's or a clipped layer's shadow),
so any style other than the new shape's own sends the commit down the full
recomposite; so does a clipped layer just above a new shape, which would clip
to it everywhere. `tst_tool_commit` holds every bounded commit to a full CPU
recomposite.

**D5 — Parallel `fill_layer`.** Its source becomes `Fn + Sync` (every caller
was already pure in the pixel; Dissolve and dither use `pixel_noise(x, y)`), and
rows run in parallel. Shape coverage sweeps only the path's columns.

## Ceilings and follow-ups

- ponytail: a shape layer's bounded composite is ~0.49 s for the 60 MP region
  on the map (the per-pixel vector-mask fill composite); compositing the fill
  layer by row runs instead of per pixel is the next step.
- Undo of a stroke or shape on the map is still 0.46–0.84 s, and of a
  full-layer gradient 2.6 s: history's whole-plane compare (`large-image-stroke-boundaries`
  ceiling) plus a full restore. Tiled planes remove both.
- Moves of a ⌀40 brush cost ~15 ms on the map against ~6 ms at 4000²; the
  difference is outside the raster and present phases the timing log records.
- Marquee's release on the map spends ~1.4 s in the command registry and panel
  refresh after the selection commits; Paint Bucket's click 5.5 s in the flood
  and the full-layer surface it reads; Gradient's 4.8 s in the full-layer fill
  and its region composite. None of these were in #719's scope; the profiler
  now measures them.
- `shape::coverage` still rasterizes on one thread (`pictura-core` has no
  thread pool); the column bound took it from 0.77 s to 0.2 s of Shape Pixels
  on the map.
