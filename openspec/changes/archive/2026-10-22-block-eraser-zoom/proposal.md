# Proposal

## Why

The Eraser's Block mode erases a 16 *document*-pixel square
(`crates/pictura-paint/src/eraser.rs`). CS6's Block is a fixed 16 *screen*-pixel
square, so its document side is 16 divided by the canvas zoom. Ours ignores the
zoom, and the size is wrong in both directions:

| zoom | CS6 | ours today |
|---|---|---|
| 25 % | 64 document pixels (16 screen pixels) | 16 document pixels (4 screen pixels) |
| 400 % | 4 document pixels (16 screen pixels) | 16 document pixels (64 screen pixels) |

At 400 % the block wipes sixteen times the area CS6 wipes; at 25 % it wipes a
sixteenth. The paint path takes no view input, so the screen-space size is
never recovered.

The Block's width also depended on where the mouse fell: the square tip's
boundary was inclusive, so a dab covered 16 pixels at a whole-pixel position
and 17 at a half-pixel one, and a one-document-pixel block — the 3200 % case —
covered two.

The Block's cursor has the same root. The canvas draws the brush-size ring for
every brush tool, so in Block mode the ring is a circle of the greyed-out
brush size, while the block itself is a fixed square. Fixing the size without
the cursor would leave the ring contradicting the erase more than it does
today.

## What Changes

- Give the Eraser's Block path the canvas zoom and convert its 16 screen
  pixels into document pixels before painting, so the block covers the same
  number of screen pixels at every zoom, as in CS6.
- Make the square tip's boundary half-open, so a Block dab covers exactly its
  diameter in document pixels whatever the dab's position in its pixel.
- Draw the Block's cursor as a square of the block's size instead of the
  brush-size circle, and resize it when the canvas zooms.

The Brush and Pencil modes are untouched: their size is in document pixels and
their ring scales with the zoom, as `tools/brush-tools` requires.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/eraser-tool`: the Block mode's size is 16 screen pixels, converted to
  document pixels at the canvas zoom, and the Block's cursor is a square of
  that size.

## Impact

- `crates/pictura-paint/src/eraser.rs` — `BLOCK_SIZE` becomes
  `BLOCK_SCREEN_PX`; `begin_erase` takes the zoom; the module doc and the
  `ponytail:` ceiling note the new rule.
- `crates/pictura-paint/src/tip.rs` — the square tip's half-open boundary, and
  the `StrokeConfig::square` doc.
- `crates/pictura-app/src/cxxqt_object/paint_tools.rs` — the `begin_eraser`
  bridge takes the zoom and exposes the block's size to C++.
- `crates/pictura-app/cpp/tool_eraser.cpp` — passes the canvas zoom.
- `crates/pictura-app/cpp/image_view.h`, `image_view.cpp` — a square brush
  outline sharing the circle's state, with a one-screen-pixel halo.
- `crates/pictura-app/cpp/tools.h`, `tools.cpp`, `tools_marquee.cpp` — the
  Block's cursor and the `refreshBrushOutline()` helper.
- `crates/pictura-app/cpp/tests/tst_mask_edit.cpp` — the two `begin_eraser`
  call sites.
- `crates/pictura-app/cpp/tests/tst_eraser_tools.cpp` and
  `crates/pictura-app/cpp/tests/CMakeLists.txt` — a new Qt Test suite.
- `docs/dev/STATE.md` — the Eraser's Block ceiling.
- No dependency, data-format, or history changes.
