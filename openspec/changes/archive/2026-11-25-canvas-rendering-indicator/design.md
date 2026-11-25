# Design

## Context

See `proposal.md` for the measurements and
`specs/ui/canvas-rendering-indicator/spec.md` for the contract. The owner chose
on #1608:
- a canvas-corner overlay, not the status bar;
- shown whenever tiles are pending, not only after restores;
- tile speed left to #1610.

An earlier plan, running restores as cancellable jobs with the progress window,
was dropped once the jump measured 9 ms.

## Decisions

### Pending means the tile worker has work

`DisplayTiles` already knows this: `in_flight` is set while a worker builds,
and `wanted` holds keys queued for the next one. `tiles_pending()` is
`in_flight || !wanted.is_empty()`. A synchronous build on the GUI thread (a
document at or under 16 Mpx, or while painting) never reports pending. That
work blocks the GUI thread, so an indicator could not animate during it anyway,
and on such a document it is short.

Panels such as the Navigator also request tiles through the same queue, so
their tiles count too. They are display tiles still on their way, and the
canvas is often waiting on the same job.

### Event-driven, with no idle timer

`ImageView::paintEvent` calls `RenderingIndicator::poke()` after it presents.
Every paint that requested tiles therefore reaches the indicator, because a
crop that misses tiles queues them during that paint. `poke()`:

- when pending and hidden: starts a single-shot 250 ms delay timer, unless it is
  already running;
- when not pending: stops the delay timer, and hides the indicator if it is
  visible.

When the delay fires, the indicator shows only if tiles are still pending. While
visible, an 80 ms animation timer advances the spinner and re-reads pending on
every tick, so the indicator hides even if no paint follows. A landing also
repaints the canvas (`pyramid_settled`), which pokes it again. When the
indicator hides, both timers stop.

The 250 ms delay matches the delay `document/long-operation-progress` uses
before its window.

### Laid over the canvas from its host

`RenderingIndicator` is a small `QWidget` that `ImageView` creates. When it
first shows, it moves to the canvas's parent (the document window's canvas
host) and sits over the canvas's top-right corner. As a child of the canvas it
would be painted into every `render()` and `grab()` of the canvas: the full gate
caught it in `tst_present_cache`'s screenshots. It follows the canvas's
move and resize through an event filter, stops when the canvas hides, and is
deleted with the canvas. A canvas without a parent (a test) keeps it as a
child. It sets `WA_TransparentForMouseEvents` and `Qt::NoFocus`, and paints a
rounded translucent badge with a rotating arc and the text "Rendering…". It lives in its own files because `image_view.cpp` and
`image_view.h` are close to the file-size target. `ImageView` gains one member
and one call.

The canvas reads the pending state through a new `LevelProvider::tilesPending`
callback, wired in `frame_canvas.cpp` to `PictureView::tiles_pending()`. It
defaults to empty, so other providers, such as tests, are unchanged.
