# Design

The move is session state, not a document edit. `TypeToolHandler` arms it on a
press outside the text (`movePending_`, with the press point and origin). Past
three screen pixels the drag sets `origin_ = moveOrigin_ + (pointer − press)`
and refreshes the overlay. Every overlay part (render, caret stops, baselines,
placement point, box polygon) derives from `origin_`, so all of it follows.
A release without a drag commits, the old behaviour. The commit reads `origin_`
like any other, so new type lands at the drop point and a reopened layer is
re-set there in its one "Edit Type Layer" state.

The cursor uses the existing pointer-driven hook. `pointerCursor()` is true while
a session is open, and `hoverCursor` returns the Move tool's cursor
(`toolCursorId(ToolId::Move)`) outside the text, box, and handles, and leaves
the I-beam elsewhere. `pointerCursorKind` reports the zone (1 move, -1 other),
which `ToolController::activeCursorKindForTest` exposes, since the offscreen
platform has no real pointer position.
