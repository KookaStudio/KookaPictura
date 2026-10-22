# Test Plan

The scenarios are the Eraser's Block mode: the engine's size rule, the zoom
that reaches it from the canvas, and the cursor drawn over it.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `Block erases a square` | Begin a Block eraser stroke on an ordinary layer at 10 % Opacity and a 100 % zoom, sample the layer under the dab and just outside it | Exactly the 16 document pixels of the square, corners included, are fully transparent, and the pixels just outside are untouched | `pictura-paint::block_erases_a_hard_square_at_full_strength`, `tst_eraser_tools::blockErasesASquareThatScalesWithTheZoom` |
| `er_block_zoom` | Zoom the canvas to 400 %, drag a Block eraser once, and count the erased document pixels; the engine's conversion is checked at 25 % and at 3200 % | The block's document side is 4 at 400 %, 64 at 25 % and 1 at 3200 %, and a dab covers exactly that many pixels along each axis, so the block is 16 screen pixels wide at the first two | `pictura-paint::the_block_square_scales_with_the_zoom`, `tst_eraser_tools::blockErasesASquareThatScalesWithTheZoom` |
| `er_block_cursor` | With the Eraser in Block mode, hover the canvas, render the canvas, and measure the dark run along the row through the pointer | The outline is a square of the block's size at the pointer, 16 screen pixels wide at a 100 % zoom, within the one-pixel tolerance a cosmetic pen's overhang needs | `tst_eraser_tools::theBlockCursorIsASquareOfTheBlockSize` |
| `er_block_cursor_zoom` | Zoom the canvas while the Eraser stays in Block mode, and measure the drawn outline again | The square is resized to the block's new size, so it stays 16 screen pixels wide | `tst_eraser_tools::theBlockCursorIsASquareOfTheBlockSize` |
