# Design

## A registry command

`help.search` is a normal registry command bound to `Shift+Space`. The
Keyboard Shortcuts editor, the menu, and `list_commands` therefore see it
without a special case, and the reserved `workspaceKeys()` list does not need
it. Text input keeps the key through Qt's `ShortcutOverride`: a line edit or
spin box accepts it for a printable key, and the type session's key filter
accepts every printable key while a type edit is open. The window's Space-pan
branch ignores a Space that carries Shift, so an unbound or rebound
Shift+Space never starts half a pan.

## The index

`buildSearchIndex(const CommandRegistry&)` is free of widgets, so the Qt Test
can drive it directly. It reads:

- `CommandRegistry::describe()` for every command; the live label replaces the
  static one, and mnemonic `&` and trailing ellipses are stripped. A command is
  runnable when it is implemented and enabled.
- The Window > Panels toggles (`panelToggles()`), which become panel entries,
  one per panel, preferring the Window menu path over the Type menu's.
- The real Preferences pages, which become preference entries (always
  runnable, since they open the dialog on that page).
- `allToolIds()` / `toolInfo()`, as `Tools > <tool>`; runnable when
  implemented.
- `searchControls()`, a curated table of panel and options-bar controls keyed
  by object name. The table is meant to grow.

The index is rebuilt every time the bar opens. It holds about 650 rows, so the
cost is negligible, and enabled state is never stale.

## Matching and ranking

The query is split on whitespace and matched case-insensitively. Every word
must occur in the label, a path segment, or a keyword. Matches rank in tiers:
exact label, label prefix, every word a word-prefix in the label, every word a
substring of the label, then matches that need the path or keywords. Within a
tier, runnable entries come first, then implemented ones, then shorter paths,
then index order. There is no fuzzy matching and no new dependency.

## The overlay

`SearchBar` is a `QFrame` child of the main window, not a `Qt::Popup`, so it
never grabs the pointer (the Brush Preset picker pattern). The query field
keeps the focus; the result list takes no focus, so Up/Down, Enter, Alt+Enter,
and Escape are handled by a filter on the field. An application event filter,
installed only while the bar is open, closes it on a press outside it. The bar
also closes when the field loses focus to anything but a popup.

## Running and revealing

The window owns the actions. A command runs through its `QAction::trigger()`,
so a checkable command toggles exactly as it does from the menu. A tool is
activated, a preference page opens the dialog, a panel is shown and outlined,
and a control's panel or tool page is brought up first and the control then
takes focus (a combo box opens its list). Reveal opens the menu bar on the
top-level menu and walks each submenu with `setActiveAction`, ending on the
item. For panels, controls, and tools, reveal outlines the target with a
short-lived `searchFlash` overlay.
