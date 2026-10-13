# Proposal

## Why

Holding `Space` to pan the canvas is unreliable: the hand cursor flashes for
about one frame and then reverts, and the pan drag is frequently lost to
whatever cursor mode was active before the press. The `tools/canvas-tools`
capability already promises that holding `Space` temporarily activates Hand
panning, but the implementation applies the override once (`ImageView::setSpacePan`)
and lets every later tool cursor/pan update overwrite it, so the override is not
actually authoritative.

## What Changes

- Make the transient `Space` hand override authoritative: while held it wins
  over every other cursor and pan update — tool hover, tool switch, and
  document/tool-state refreshes — not just the initial press.
- Keep the override alive for the whole hold: a key auto-repeat release (X11
  emits synthetic `KeyRelease` events while a key is held) SHALL NOT end the
  override. The `Space` release handler is guarded with `!isAutoRepeat()`,
  matching the press handler and the existing `LiquifyCanvas` space handling.
- Preserve the active tool and its state; releasing `Space` restores the
  previous tool cursor and pan-enable state exactly.
- Suppress pointer-hover tool updates while `Space` is held with no mouse button
  down, so hover cannot clobber the hand cursor; a tool drag already in progress
  still runs to completion.
- Ensure a pan press cannot be dropped when a cursor-mode update lands mid-hold.
- Add a Qt Test regression covering the cursor staying on the hand and the pan
  surviving a tool-state refresh.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `tools/canvas-tools`: the "Space temporarily activates Hand panning"
  requirement gains explicit precedence — while `Space` is held the override
  SHALL take precedence over every other cursor/pan update, and a cursor-mode
  update mid-hold SHALL NOT drop the pan or revert the hand cursor.

## Impact

- `crates/pictura-app/cpp/image_view.h` / `image_view.cpp` — override guards in
  `setPanEnabled`, `mousePressEvent`, `mouseMoveEvent`.
- `crates/pictura-app/cpp/tools_marquee.cpp` — top-level override check in
  `ToolController::refreshCursor`, placed immediately after the `canvas_` null
  check and before the `canvasSampler_` early return.
- `crates/pictura-app/cpp/frame.cpp` — guard the Space key-release against
  auto-repeat so a synthetic release cannot clear the override mid-hold.
- `crates/pictura-app/cpp/tests/tst_canvas_pan.cpp` — regression test
  (`tst_canvas_pan` is already registered in `crates/pictura-app/cpp/tests/CMakeLists.txt`).
- No Rust, no new dependencies, no document or codec behavior change.
