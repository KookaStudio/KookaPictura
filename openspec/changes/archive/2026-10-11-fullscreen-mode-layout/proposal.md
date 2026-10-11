# Full screen mode layout and Tab chrome

## Why

Issue #328 follow-up: testing the advisory-message build surfaced two
divergences from CS6.

- `View > Screen Mode > Full Screen Mode With Menu Bar` entered an empty
  layout — the mode hid the Tools panel and every panel. CS6 keeps the Tools
  panel, Options bar, and panels in this mode and drops only the
  document-window chrome (tab, scroll bars, status bar, window buttons).
- In Full Screen Mode, `Tab` revealed the panels but not the menu bar, so the
  top menu stayed unreachable without leaving the mode.

## What Changes

- Full Screen Mode With Menu Bar SHALL keep the Tools panel, Options bar, and
  panels visible (subject to the persisted Tab state), hiding only the
  document-window chrome.
- In Full Screen Mode, `Tab` SHALL reveal both the panel columns and the menu
  bar on its first press and hide them again on the next; the reveal stays
  transient.
- `Tab` in Full Screen Mode With Menu Bar SHALL toggle the same persisted panel
  visibility as in Standard Screen Mode.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `ui/screen-modes`: the "Tab reveals hidden full-screen chrome" requirement now
  fixes the Full Screen Mode With Menu Bar layout and makes `Tab` reveal the
  menu bar in Full Screen Mode.

## Impact

- `crates/pictura-app/cpp/frame.{h,cpp}` — `setScreenMode`,
  `togglePanelsHidden`.
- `crates/pictura-app/cpp/tests/tst_screen_modes.cpp` — the layout checks and a
  new `fullWithMenuBarKeepsPanels` case.
- `docs/02-ui-ux/application-frame.md` — the Tab/full-screen chrome contract.
- No new dependency.
