# Tasks

## 1. Tests first

- [x] 1.1 Add `tst_ui_scale` covering the scale contract, font/QSS, frame
  chrome, canvas isolation, session persistence, Window commands, and the
  Interface control; register it in the tests CMake list.

## 2. Theme and font

- [x] 2.1 Add the UI scale choices, normalization, percent accessors, and
  `scaled()` to `Theme`.
- [x] 2.2 Scale literal pixel lengths in the stylesheet and the token metrics,
  leaving `shade-step` alone.
- [x] 2.3 Scale `bundledUiFont()` from the pinned 12 px base.

## 3. Frame and chrome

- [x] 3.1 Persist `uiScalePercent` in `SessionState` and restore it before the
  chrome is built.
- [x] 3.2 Add the live setter and re-scale fonts, QSS, status bar, menu-corner
  icons, tool slots, panel columns, and panel groups.
- [x] 3.3 Scale panel-column bounds/defaults and the theme border constants
  used by widgets.

## 4. Commands and Preferences

- [x] 4.1 Register the Window readout, Enlarge, Decrease, and Reset commands
  with handlers, enablement, and label providers.
- [x] 4.2 Add the Interface UI Size combo and keep it in step with the frame.

## 5. Verification

- [x] 5.1 Run `cargo fmt --all`, the app build, and the Qt tests.
- [x] 5.2 Run `openspec validate --all --strict` and archive the change in the
  shipping PR.
