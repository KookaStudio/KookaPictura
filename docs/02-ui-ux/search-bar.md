# Search Bar (Help > Search)

- **Spec ID:** `UI-013`
- **Status:** `Spec'd`
- **Parity tier:** `Non-parity (post-CS6, owner decision)`. CS6 has no command
  search; Kooka Pictura adds one so a user can reach any entry point without
  knowing where it lives in the CS6 menu tree (#788).
- **New in CS6:** `No`. Not a CS6 feature.
- **Depends on:** `02-ui-ux/menus.md` (`UI-002`), `02-ui-ux/keyboard-shortcuts.md`
  (`UI-011`), `02-ui-ux/toolbox-and-options-bar.md` (`UI-004`),
  `02-ui-ux/preferences.md` (`UI-010`), OpenSpec capability `ui/search-bar`.

> CS6 has nothing to match here: this page describes Kooka Pictura's own
> behavior. The normative requirements live in
> `openspec/specs/ui/search-bar/spec.md`; this page is the user and developer
> guide to them.

## CS6 behavior

None. CS6 has no search over its menus, tools, or panels.

### Kooka Pictura behavior: using the search bar

1. Press **Shift+Space**, or choose **Help > Search…**. A search bar opens over
   the upper third of the canvas (over the empty workspace when no document is
   open), with the cursor in its query field.
2. Type a word or two, for example `warp`. The list shows every matching entry
   with its location as a breadcrumb and its shortcut on the right:

   ```
   Edit > Transform > Warp
   Edit > Transform Path > Warp
   Type > Warp Text
   ```

3. Use **Up** and **Down** to choose a row, then:
   - **Enter** (or a click on the row) **runs** it. A command runs as if picked
     from its menu, a tool becomes active, a panel is shown, a Preferences page
     opens, and a panel control is brought up and takes the keyboard focus (a
     drop-down opens its list).
   - **Alt+Enter** **reveals** where it lives without running it. A command's
     menus open with the item highlighted (for `Edit > Transform > Warp`, the
     Edit menu opens, then its Transform submenu, with Warp active). A tool's
     toolbox slot, a panel, or a panel control is outlined with a short pulse.
4. **Escape**, a click outside the bar, or moving the focus elsewhere closes
   it. Focus returns to where it was.

Greyed rows are entries that cannot run right now: either the command is not
implemented yet, or it is disabled in the current state (for example Warp with
no document open). Enter and a click do nothing on a greyed row; Alt+Enter
still opens its menus, so the search also teaches where a feature will live.
Qt never highlights a disabled menu item, so the greyed item itself is not
highlighted, and the reveal does not enable it (Enter in the open menu would
then run it). The row's tooltip says which case applies.

Matching ignores case, and every word you type must match. `transform warp`
finds `Edit > Transform > Warp` but not `Type > Warp Text`. A word can match the
entry's name, any step of its location, or a few built-in keywords: `blend`
finds the Layers panel's Blend Mode and the options bar's Mode, `lock` finds the
Layers panel lock buttons, and `settings` finds the Preferences pages.

The key stays out of the way of typing:

- In any text field (a layer name, a number box, the search field itself) the
  field keeps Shift+Space; a field that accepts spaces types one.
- While a type edit is open on the canvas, Shift+Space types a space into the
  text.
- A bare **Space** still pans the canvas. A Space pressed with Shift never
  starts a pan.

The status bar's right end shows a **Search** keycap with the current binding.
Rebind it in **Edit > Keyboard Shortcuts** (Application Menus, `Help >
Search…`); the keycap follows. With no binding the keycap is hidden and the
menu item still works.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Help > Search…` | Menu command (`help.search`) | `Shift+Space` | Rebindable in Edit > Keyboard Shortcuts. |
| Search bar over the canvas | In-window overlay | — | Query field, result list, key legend. |
| Result row | List row | — | Breadcrumb path; shortcut at the right; greyed when it cannot run. |
| Status bar, right end | Keycap hint | — | `Search` with the live binding. |
| Reveal outline | Transient overlay | `Alt+Enter` | Accent outline pulsed over a tool slot, panel, or control. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Query | Text | empty | any text | Split on whitespace; every word must match. |
| Results shown | Count | up to 50 | 0–50 | At most 10 rows visible; the list scrolls. |
| Search shortcut | Key binding | `Shift+Space` | any free binding, or none | Edit > Keyboard Shortcuts. |

## Algorithms & pipeline

**The index.** Each time the bar opens it rebuilds its index from the live
application, so labels, enabled state, and shortcuts are never stale. The index
holds about 650 entries and builds in well under a frame. Its sources:

| Kind | Source | Path shown | Runs when |
|---|---|---|---|
| Command | Every command in the command registry (`CommandRegistry::describe()`), with its live label | Its menu path, e.g. `Edit > Transform > Warp` | Implemented and enabled |
| Panel | The Window > Panels toggles (`panelToggles()`), one entry per panel; the Window menu path wins over Type > Panels. A panel with no toggle (the placeholder Styles panel) is found only as its greyed menu command | `Window > Panels > Layers` | Always (shows the panel) |
| Preferences page | The dialog's real pages | `Edit > Preferences > Units & Rulers` | Always (opens the page) |
| Tool | The tool catalogue (`allToolIds()` / `toolInfo()`) | `Tools > Magic Wand` | The tool is implemented |
| Panel control | The curated table `searchControls()` | `Layers panel > Blend Mode`, `Options bar > Brush > Flow` | Always |

Mnemonic `&` marks (a doubled `&&` stays one `&`), an inline tab shortcut, and
trailing ellipses are stripped from names. `Help >
Search…` itself is left out.

**Matching and ranking.** The query is lower-cased and split on whitespace
(spaces, tabs, newlines). An
entry matches when each word occurs as a substring of its name, of a path step,
or of a keyword. Matches are ranked in tiers:

1. the name equals the whole query;
2. the name starts with the query;
3. each word starts a word of the name;
4. each word occurs inside the name;
5. some word matches only the path or a keyword.

Within a tier, entries that can run come first, then implemented ones, then
shorter paths, then catalogue order. There is no fuzzy or typo-tolerant
matching.

**Run.** A command runs through its menu `QAction`, so a checkable command
toggles exactly as it does from the menu, and a disabled one does nothing. A
panel is shown in whichever column holds it. A panel control first brings up
its context: it activates the tool whose options page holds it, or shows its
panel. It then focuses the control: a drop-down opens, and a number field
selects its text.

**Reveal.** For a command, the menu bar opens the top-level menu and then each
submenu in turn, ending with the item active; a disabled item opens its
menus without a highlight. For a tool, panel, or control,
a 2 px accent outline pulses over it for about a second.

## Rust module mapping

None. The search bar is UI glue in the Qt shell, with no engine or document
state.

## Qt6 component mapping

- `SearchBar` (`crates/pictura-app/cpp/search_bar.{h,cpp}`) — a `QFrame`
  overlay that is a child of the main window, not a `Qt::Popup`, so it never
  grabs the pointer. A `QLineEdit` holds the focus, and the result
  `QListWidget` takes none. An application event filter, installed only while
  the bar is open, handles the keys and an outside press.
- `buildSearchIndex()` / `searchIndex()` (`search_index.{h,cpp}`) — the
  widget-free index and ranking, tested directly.
- `searchControls()` (`search_controls.cpp`) — the curated control table.
- `PicturaMainWindow` (`frame_search.cpp`) — the `help.search` handler, run
  and reveal, the status-bar keycap (a `ToolHintBar`), and `flashWidget()`.
- QSS: `QFrame#searchBar`, `QListWidget#searchResults`, `QWidget#searchFlash`
  in `theme.cpp`, from the design-system tokens.

### Keeping the index complete

What a new feature needs in order to be found:

| You add… | Found automatically? | What to do |
|---|---|---|
| A menu command | Yes | Register it in the command registry as usual. |
| A tool | Yes | Add its `ToolId` and its row in the engine tool catalogue (`pictura_script::tool_catalogue`); it runs once `implemented` is true. |
| A panel | Yes, once it has a Window > Panels toggle | Add it to `panelToggles()` (its menu toggle needs that anyway). |
| A Preferences page | No | Add the page to the list in `buildSearchIndex()`. |
| A panel or options-bar control | No | Add a row to `searchControls()`: container and control object names, the tool whose options page holds it (or none), the breadcrumb, and extra keywords. `tst_search_bar::curatedControlsExist` fails if a row names a widget that does not exist. |

## Data-model impact

None. The binding is stored with the other shortcut overrides in the session.

## Edge cases

- **No document open.** The bar sits over the empty workspace. Document
  commands are listed greyed.
- **A pending transform.** Commands the transform locks are greyed, exactly as
  in the menus.
- **Text focus.** A text field or a type edit keeps Shift+Space; the bar never
  opens from one.
- **Closing.** Closing moves focus back before hiding the bar, so a hidden field
  never keeps the focus and swallows the next Shift+Space.
- **Rebound or unbound.** Shift+Space with no command bound does nothing and
  does not start a Space pan.
- **Hidden menu bar** (full-screen modes). Reveal of a command does nothing
  visible; run still works.
- **A label provider's doubled ellipsis** (`Liquify……`) is stripped to the bare
  name.

## Parity acceptance criteria

None for CS6 parity (non-parity feature). The behavioral contract is the
`ui/search-bar` OpenSpec capability, covered by `tst_search_bar`:

- Shift+Space opens the bar with the field focused; it reopens after Escape.
- `warp` lists `Edit > Transform > Warp` first; Enter starts a Warp session.
- Alt+Enter on Warp opens Edit > Transform with Warp active.
- A stub row is greyed and Enter on it runs nothing; Alt+Enter opens its menus.
- Enter shows a panel, opens a Preferences page, or brings up and focuses a
  control; Alt+Enter outlines a tool slot, panel, or control without running it.
- Up and Down stop at the first and last rows; losing focus to a widget other
  than a popup closes the bar.
- Every curated control resolves to a live widget.

## Sources

- GitHub issue #788 — the request and the owner's decisions (index scope,
  run/reveal keys, greyed stubs, the Shift+Space binding).

## Open questions

- Should typo-tolerant (fuzzy) matching be added? Resolve by user feedback on
  missed searches.
- Should the Preferences page list be read from the dialog instead of kept in
  the index? Resolve when a new page is added.
