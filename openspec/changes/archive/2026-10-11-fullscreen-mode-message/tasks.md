# Tasks

## 1. Advisory dialog and session flag

- [x] 1.1 Add `bool showFullScreenMessage = true` to `SessionState` in `crates/pictura-app/cpp/session.h` and load/save it as `showFullScreenMessage` in `session.cpp` (default true; off after "Don't show again").
- [x] 1.2 Add `crates/pictura-app/cpp/screen_mode_message.h`/`.cpp` with `bool confirmFullScreenMode(QWidget* parent)`: a `Message`-titled modal with the CS6 body text, a default `Full Screen` (accept) button, `Cancel` (reject), and a `Don't show again` checkbox that persists the suppression. Register both files in `CMakeLists.txt`.
- [x] 1.3 Verify a Qt Test asserts the dialog's title, body, button labels, and checkbox; Cancel aborts and the checkbox persists the flag.

## 2. Request path and keys

- [x] 2.1 Add `requestScreenMode(ScreenMode)` and a `nextScreenMode(bool)` helper in `frame.{h,cpp}`; `requestScreenMode` prompts only on Standard → full screen (design D1/D3) and delegates to `setScreenMode`; `cycleScreenMode` keeps using `nextScreenMode` and stays prompt-free.
- [x] 2.2 Point the three `View > Screen Mode` handlers in `frame_menus.cpp` at `requestScreenMode`, and the `F` key in `keyPressEvent` at `requestScreenMode(nextScreenMode(...))`.
- [x] 2.3 Add the `Esc` fallback in `keyPressEvent`: after the tool-cancel check, `Esc` in a full-screen mode returns to Standard (design D4).
- [x] 2.4 Make the `Tab`/`Shift+Tab` shortcuts toggle the transient `fullScreenChromeShown_` in a full-screen mode (first press reveals) while Standard mode keeps toggling `panelsHidden_` (design D5). Verify `setPanelsHidden` remains the Standard-mode/global setter.
- [x] 2.5 Verify a Qt Test drives `F`, `Esc`, and `Tab` through `QTest::keyClick` and checks the mode, the panel visibility, and the transient restore.

## 3. Control server and docs

- [x] 3.1 Add the three screen-mode command ids to `isModalCommand` in `control_server.cpp` so `dispatch_command` refuses them.
- [x] 3.2 Update `docs/02-ui-ux/application-frame.md`: the advisory dialog, its persistence, `Esc`/`F` return, and the first-`Tab` reveal in full screen; note the issue screenshot as the source.
- [x] 3.3 Update `tst_command_tree.cpp`'s toolbox screen-mode check to suppress the advisory through the session store before triggering the action.

## 4. Verification

- [x] 4.1 Register `tst_screen_modes` in `crates/pictura-app/cpp/tests/CMakeLists.txt`; run `cmake --build build --parallel` and `ctest --test-dir build -R '^tst_' --output-on-failure`.
- [x] 4.2 Run `./build/pictura --headless --self-test` and `bash scripts/check-selftest-budget.sh`.
- [x] 4.3 Run `openspec validate --all --strict`, `bash scripts/check-file-size.sh`, and `bash scripts/guard.sh`.
- [x] 4.4 Run `bash scripts/verify-fast.sh` and confirm no regressions.
