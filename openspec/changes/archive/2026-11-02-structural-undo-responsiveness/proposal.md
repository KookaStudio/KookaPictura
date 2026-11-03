# Proposal

## Why

Issue #1098. Any restore reporting the whole document rebuilt the canvas on the
GUI thread. On a 16507×16196 document that cost 410–470 ms for undo and over
700 ms for a history jump, even when no pixel changed. Most of those restores
did not need it: only geometry, mode and depth changes touch every pixel.

## What Changes

- **Restores report what they changed.** An unchanged composite reports an empty
  region, document channels no longer widen a restore, and a mask change no
  longer does either, because the canvas shows a mask only while it is soloed.
- **History jumps use the stored composite.** A whole-document jump no longer
  composites the layers again.
- **The pyramid of a large whole-document restore builds off-thread.** Undo
  returns with level 0 and a coarse level, the canvas presents the coarse level,
  and the exact pyramid replaces it when the worker lands. Anything that needs
  the exact pyramid sooner builds it synchronously.

## Capabilities

### Modified Capabilities

- `document/history-restore-responsiveness`
- `document/in-place-history`

## Impact

`crates/pictura-app` history and display paths, `crates/pictura-render`
`ViewPyramid`, one new `pyramidSettled` signal on `PictureView`.
