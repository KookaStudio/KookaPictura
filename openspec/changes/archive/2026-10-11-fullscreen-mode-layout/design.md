# Design — full screen mode layout and Tab chrome

## Context

`PicturaMainWindow::setScreenMode` is the single choke point for the three
screen modes; `togglePanelsHidden` is the Tab/Shift+Tab target. The advisory
change introduced `fullScreenChromeShown_` as a transient panel reveal for both
full-screen modes, which hid the panels in Full Screen Mode With Menu Bar and
left the menu bar out of the Full Screen Mode reveal.

## Decisions

- **D1 — Full Screen With Menu Bar keeps the standard panel state.** The mode
  applies `setPanelColumnsVisible(!panelsHidden_)`, the same state Standard
  uses, and never consults the transient flag. CS6 hides only the
  document-window chrome here; the docs table in `application-frame.md` already
  says the Tools panel and panels are retained.
- **D2 — The transient reveal is Full Screen Mode only.** `fullScreenChromeShown_`
  now controls the menu bar as well as the panel columns, resets on every entry
  to Full Screen Mode (but survives re-selecting the active mode), and is
  applied by `togglePanelsHidden` only in that mode.
- **D3 — Tab in Full Screen With Menu Bar is the persisted toggle.** The mode
  shares Standard's `panelsHidden_` so the Tab state survives the mode round
  trip, matching CS6's "works in all Screen Modes" shortcut.
- **D4 — The menu bar toggle re-lays the workspace frame.**
  `layoutWorkspaceFrame` positions the 3 px band frame from the menu bar's
  geometry, so the Full Screen reveal calls it after toggling.

## Alternatives

- A transient reveal in Full Screen With Menu Bar too: rejected — the mode is
  defined by keeping the panels, and CS6's `Tab` there is the ordinary toggle.
- Keeping `fullScreenChromeShown_` panel-only and revealing the menu bar with a
  second flag: rejected — one flag describes one reveal.
