# Tasks: paragraph-styles-panel

- [x] Add paragraph-style read-back bridge functions to
      `cxxqt_object/type_tools.rs` (`type_paragraph_style_count`,
      `type_paragraph_style_name`, `type_paragraph_style_character`,
      `type_paragraph_style_paragraph`, `type_paragraph_style_font`).
- [x] Extend the type model with the CS6 style attributes the full dialog needs
      (faux bold/italic, the ten OpenType features, language, vertical Roman
      alignment, auto leading, the hyphenation dictionary) and read/author their
      EngineData keys.
- [x] Add `paragraph_style_dialog.{h,cpp}` +
      `paragraph_style_dialog_pages.cpp` with the style name, Preview, and all
      seven CS6 pages.
- [x] Add `type_preview_paragraph_style` (a no-history style edit) and wire the
      dialog's Preview / Cancel-restore through the panel.
- [x] Add `panels/paragraph_styles_panel.{h,cpp}` (list, apply on click, edit on
      double-click, footer create/delete, protected default).
- [x] Register the new sources in `CMakeLists.txt` and include them from
      `frame_includes.h`.
- [x] Add the panel to `frame.h` / `frame_build.cpp` (type group, hidden by
      default, refreshed on retarget and Layers selection).
- [x] Turn the two disabled command leaves into enabled checkable commands with
      panel toggles (`commands.h`, `command_tree.cpp`, `frame_menus.cpp`).
- [x] Add Qt Test `tst_paragraph_styles_panel` (list/apply, create/delete, menu
      toggles, the seven dialog pages, attribute round-trip, and preview /
      cancel-restore) and register it in `cpp/tests/CMakeLists.txt`.
- [x] Run `bash scripts/verify-fast.sh`.
