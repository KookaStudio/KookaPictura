# Test plan: canvas-rendering-indicator

The indicator's timing is tested in a Qt Test suite whose canvas reads its
pending state from a test flag through `LevelProvider::tilesPending`, so no test
depends on how fast a worker builds tiles. The view's own pending state is read
from a real view of a document large enough to build its tiles on a worker; the
request sets it synchronously, so the check right after it is deterministic. Seeing the indicator over
a real slow redraw is the one manual row.

| Scenario | Steps | Expected result | Coverage |
| --- | --- | --- | --- |
| `cri_shows` | Set the pending flag, repaint the canvas, wait 400 ms. | The indicator is visible and its animation timer is active. | `tst_rendering_indicator::showsWhenTilesStayPending`; manual: jump back 12 History states on a 21000² document with strokes and see "Rendering…" in the canvas corner until the canvas updates |
| `cri_hides` | With the indicator visible, clear the flag and let one animation tick pass. | The indicator hides. | `tst_rendering_indicator::hidesWhenTilesLand` |
| `cri_fast` | Set the flag, repaint, clear the flag after 100 ms, repaint, wait 400 ms. | The indicator never became visible. | `tst_rendering_indicator::fastRedrawShowsNothing` |
| `cri_transparent` | With the indicator visible, press the mouse at its centre. | The press reaches the canvas, not the indicator. | `tst_rendering_indicator::passesMouseThrough` |
| `cri_not_in_render` | Put a canvas in a host widget, render it idle, make tiles pending until the badge shows, render it again. | The badge is a child of the host over the canvas's top-right corner, and the two renders are identical. | `tst_rendering_indicator::staysOutOfCanvasRenders` |
| `cri_idle` | Show the indicator, clear the flag, let it hide, then check its timers. | Neither the delay nor the animation timer is active. | `tst_rendering_indicator::noTimerWhileIdle` |
| `cri_pending` | On a 5000 × 4000 document (over the 16 Mpx synchronous limit), crop level 0 through `canvas_display_image` before its tiles exist, read `tiles_pending`, then wait for the landing. | Pending is true right after the crop and false once the tiles have landed. | `tst_rendering_indicator::requestedTilesArePendingUntilTheyLand` |
