# Proposal

## Why

The workspace scrollbars run backwards: the handle travels the *same* way as the
content, so dragging the horizontal bar right slides the canvas right and
reveals the *earlier* part of the image. The vertical bar is upside down — the
top of the bar shows the bottom of the picture. The bars cannot be used to
navigate: pushing the handle the way you want to look moves the canvas away
(#823).

The bars' *range* is right; only the projection is wrong. The range is the same
one that clamps the canvas pan, so it covers every position the canvas can be
reached to — including the reveal margin, where the image is partly off the
workspace. The value is projected from the offset in the wrong direction: it
counts from the range's minimum, so raising the bar's value raises the offset
and the content travels with the handle.

## What Changes

- The value↔offset projection inverts. A bar's value becomes the distance from
  the shared range's maximum, so the handle travels **against** the content:
  the bar's minimum shows the start of the document, its maximum shows its end,
  and raising the value lowers the canvas offset.
- Nothing else moves. The bars keep the shared pan range, so the handle still
  reaches every pannable position — the reveal margin included — and it rests
  at the end of its track only when the canvas has reached the pannable limit.
  The canvas offset keeps its meaning for the pan, the zoom, the rulers and the
  Navigator.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/canvas-scrollbars`: a new requirement fixes the direction — the
  handle travels against the content, the bar's ends are the document's ends,
  and the bar covers the reveal margin.

## Impact

- `crates/pictura-app/cpp/canvas_scrollbars.cpp` — the four value↔offset
  expressions.
- `crates/pictura-app/cpp/canvas_scrollbars.h` — the class comment, which
  promises a projection of the canvas offset without saying in which direction.
- `crates/pictura-app/cpp/selftest_canvas_view.cpp` — the
  `canvas_scrollbar_roundtrip` check (exit code 309) asserts the old
  projection.
- `crates/pictura-app/cpp/tests/tst_canvas_pan.cpp` — two new Qt Test cases
  for the range and the direction.
- `docs/dev/STATE.md` — the direction rule.
- No dependency, data-format, API, or history changes. The pan clamp, the
  zoom, the rulers and the Navigator keep the offset's meaning and are
  untouched.
