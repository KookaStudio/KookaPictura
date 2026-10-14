# Tasks

## 1. Browser

- [x] 1.1 `Theme::currentLevel()` in `theme.{h,cpp}`.
- [x] 1.2 `DesignSystemDialog` (`design_system_dialog.{h,cpp}`): story tree
  (grouped, filterable), lazy page stack, per-story title/description/QSS
  selector, token readout, brightness combo, "show states" toggle.
- [x] 1.3 `designSystemStories()` + page builders
  (`design_system_dialog_pages.cpp`): Foundations (palette, brightness, metrics,
  typography, states), Shell (menu bar, menu, options bar, tools column, panel
  group/tabs, document tabs, floating panel, status bar, splitter), Controls
  (the curated primitives), Patterns (dialog, layers, colour, docked column) —
  all real widgets.
- [x] 1.4 Live brightness preview (`Theme::apply`) with restore on close.

## 2. Wiring

- [x] 2.1 `command_ids::HelpDeveloperDesignSystem` and the
  `Help ▸ Developer ▸ Design System…` command in `command_tree.cpp`.
- [x] 2.2 Handler + `PicturaMainWindow::showDesignSystemDialog()` in
  `frame_menus.cpp`/`frame.h` (single modeless instance).
- [x] 2.3 Register the sources in `CMakeLists.txt`.

## 3. Test

- [x] 3.1 `tst_design_system_dialog.cpp`: stories/groups, every page builds,
  curated primitives present, filter narrows, brightness previews and restores,
  pages render; registered in `crates/pictura-app/cpp/tests/CMakeLists.txt`.

## 4. Docs

- [x] 4.1 Note the Developer entry in the Help-menu doc (`TASK-ALLOWS-DOCS`).
