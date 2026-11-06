# Grow and trim pixel layers

## Why

Issue #1208, step 3 of #1209. A pixel layer is stored at a fixed rect: a new
layer is canvas-sized, and no paint, fill or filter engine grows a layer's rect.
A small mark on a 17000×17000 layer therefore costs the whole canvas (1.16 GB of
planes) in the layer, in history anchors, and in every whole-layer read. #1207
bounded the fill, Move and Free Transform work to their own boxes but kept
layers canvas-sized: trimming a layer to its content on Move would make later
strokes, fills and filters outside the trimmed rect do nothing.

## What Changes

- **A new pixel layer is empty.** `Layer > New Layer` stores a 0×0 rect with no
  pixel planes; reading outside the rect is transparent. A New Layer dialog
  asking for a neutral fill stays canvas-sized, because the fill is its content.
- **Edits grow their target.** A brush stroke, a fill, a filter, Move and Free
  Transform grow the target pixel layer's rect to cover the damage, padding with
  transparent pixels. The added bytes are reserved through the memory governor.
  A transparency-locked layer never grows (its content cannot reach outside its
  pixels).
- **Cheap trims follow content.** A Move commit, a Free Transform session and a
  layer created from a fill trim the layer to its non-transparent content. The
  composite is unchanged; only fully transparent padding is dropped.
- **PSD keeps the stored rect.** An empty layer writes a 0×0 record and a grown
  layer writes its grown rect; existing files still round-trip byte-identically.
- **The fill memory gate uses the grown rect**, so its band estimate matches the
  planes the fill will actually fork.

## Capabilities

### New Capabilities

- `document/layer-content-bounds`
- `codec/empty-layer-rect`

### Modified Capabilities

- `tools/fill-memory-bounds`: the band estimate is taken at the layer's grown
  rect, not its old rect.

## Impact

`pictura-core` (a `layer_bounds` module that grows channels and retained native
samples through the governor), `pictura-render` (empty new layers, trim helpers,
filter growth), `pictura-paint` (stroke and fill growth), `pictura-app` (the
fill, filter and Move call sites, the memory gate), `pictura-codec` (0×0 layer
records and the grown-rect round trip).
