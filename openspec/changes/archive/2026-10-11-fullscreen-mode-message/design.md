# Design — full screen mode advisory message

## Context

`PicturaMainWindow::setScreenMode` is the single choke point for the three
screen modes; the menu handlers, the Tools-panel button (through the same
registry actions), and the `F` key (`cycleScreenMode`) all funnel into it. The
self-test and the Qt suites drive `setScreenMode`/`cycleScreenMode` directly as
state setters, so the prompt cannot live inside them without blocking the
headless oracle.

## Decisions

- **D1 — The prompt is a request-path concern.** A new
  `requestScreenMode(ScreenMode)` shows the advisory when the transition is
  Standard → full screen and the session flag allows it, then delegates to
  `setScreenMode`; Cancel returns without a mode change. The menu handlers and
  the `F` key (through a small `nextScreenMode` helper) use the request path.
  `setScreenMode` and `cycleScreenMode` stay prompt-free state setters for the
  self-test, the control server, and the tests.
- **D2 — The advisory reads the session store per call.** Like
  `confirmLiveShapeToPath`, it loads the session each time, so the test suites
  can suppress it by writing the flag before triggering a command and the
  "Don't show again" choice is picked up without restarting.
- **D3 — The prompt fires only on Standard → full screen.** Switching between
  the two full-screen modes, or re-selecting the active mode, never prompts.
- **D4 — `Esc` is handled after the tools.** `cancelPolygonLasso` is the
  established generic tool-cancel hook (it covers crop, perspective crop,
  lasso, and the rest), and a transform session consumes `Esc` in the canvas;
  the frame's fallback then returns a full-screen frame to Standard.
- **D5 — The full-screen `Tab` reveal is transient.** A `fullScreenChromeShown_`
  flag starts false on each Standard → full transition; `Tab`/`Shift+Tab` toggle
  it and the panel columns' visibility. Exiting to Standard re-applies the
  persisted `panelsHidden_` preference, so revealing panels inside full screen
  does not silently change the standard layout.
- **D6 — The control server refuses the three commands.** The dialog makes them
  modal-opening, so `isModalCommand` gains them; this follows the documented
  `refused` contract instead of risking a hung socket.

## Alternatives

- Prompt inside `setScreenMode`: rejected — the self-test and several Qt suites
  call it headlessly and would block on `exec()`.
- A non-modal overlay hint in the canvas: rejected — CS6 uses a modal message
  box with a "Don't show again" checkbox (issue screenshot), and the Cancel
  button has to abort the transition.
- Remembering the last full-screen mode for `F`: rejected — CS6 cycles through
  all three modes, which the existing requirement already fixes.
