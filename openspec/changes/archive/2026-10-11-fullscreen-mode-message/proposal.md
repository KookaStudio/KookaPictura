# Full screen mode advisory message

## Why

Photoshop CS6 announces the first entry into a full-screen mode with a "Message"
dialog: panels are hidden, they can be reached from the screen edges or with
`Tab`, and `F` or `Esc` returns to Standard Screen Mode; a "Don't show again"
checkbox suppresses it. Kooka Pictura switches modes silently, `Esc` does not
return to Standard even though the docs promise it, and the first `Tab` in a
full-screen mode hides (rather than reveals) the panels because the Tab state
starts out of sync with the mode's hidden chrome (#328).

## What Changes

- Entering Full Screen Mode or Full Screen Mode With Menu Bar from Standard
  Screen Mode shows the CS6 advisory: a "Message" dialog with the documented
  body text, a default **Full Screen** button, a **Cancel** button that aborts
  the switch, and a **Don't show again** checkbox whose choice persists in the
  session store.
- No prompt when switching between the two full-screen modes or when re-entering
  full screen after the checkbox was set.
- `Esc` from either full-screen mode returns to Standard Screen Mode once no
  tool has claimed the key (transform sessions, crop, polygon lasso, and text
  editing keep their cancel priority).
- `Tab` in a full-screen mode reveals the hidden panels on its first press and
  hides them again on the next; the reveal is transient and does not change the
  Standard-mode panel preference.
- The three `View > Screen Mode` commands join the control server's
  modal-command denylist, so `dispatch_command` refuses them instead of hanging
  on the dialog.

## Capabilities

### New Capabilities

- `ui/screen-modes`: the full-screen advisory dialog and its persistence, the
  `F`/`Esc` return to Standard, and the `Tab` reveal of hidden full-screen
  chrome.

### Modified Capabilities

<!-- None: the cycling and canvas-colour contract in `ui/application-shell`
     does not change; the new behavior is additive. -->

## Impact

- `crates/pictura-app/cpp/screen_mode_message.{h,cpp}` — the advisory dialog.
- `crates/pictura-app/cpp/frame.{h,cpp}`, `frame_session.cpp`,
  `frame_menus.cpp` — request path, `Esc`, and the transient `Tab` reveal.
- `crates/pictura-app/cpp/session.{h,cpp}` — the `showFullScreenMessage` flag.
- `crates/pictura-app/cpp/control_server.cpp` — the three screen-mode commands
  in the modal denylist.
- `crates/pictura-app/cpp/tests/tst_screen_modes.cpp` — new Qt Test suite;
  `tst_command_tree.cpp` — the toolbox screen-mode check disables the prompt.
- `CMakeLists.txt`, `crates/pictura-app/cpp/tests/CMakeLists.txt` — the new
  sources.
- `docs/02-ui-ux/application-frame.md` — the advisory and the full-screen keys.
- No new dependencies.
