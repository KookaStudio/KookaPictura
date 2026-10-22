# Test plan — Shift+Space search bar

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `sb_open` | Show the window, focus the canvas host, press Shift+Space | Bar visible, empty query, query field focused | `tst_search_bar::shiftSpaceOpens` |
| `sb_hint` | Read the status hint; rebind `help.search` to Ctrl+K | Hint shows Shift+Space, then Ctrl+K | `tst_search_bar::rebindUpdatesHint` |
| `sb_textfield` | Focus a QLineEdit in the window, press Shift+Space | The field gains a space; bar stays closed | `tst_search_bar::textFieldKeepsShiftSpace` |
| `sb_typeedit` | Open a type edit on a document, press Shift+Space | The edit's text gains a space; bar stays closed | `tst_search_bar::typeEditKeepsShiftSpace` |
| `sb_space` | Send Space, release; send Shift+Space to the window's key handler | Space starts the pan; Shift+Space does not | `tst_search_bar::spaceStillPans` |
| `sb_sources` | Build the index from the window's registry | Command, tool, panel, preference and control entries present; each panel once | `tst_search_bar::indexCoversSources` |
| `sb_controls` | Resolve every curated control in the window | Container and control widget found for each row | `tst_search_bar::curatedControlsExist` |
| `sb_warp` | Search `warp` | First hit is `Edit > Transform > Warp` | `tst_search_bar::warpFindsTransformWarp` |
| `sb_rank` | Search `transform warp`, `layers`, and a synthetic index | AND across words; exact > prefix > path-only; runnable first | `tst_search_bar::rankingOrder` |
| `sb_run` | Open a document, search `warp`, press Enter | Warp session starts; bar closes | `tst_search_bar::enterRuns` |
| `sb_stub` | Search a stub command, press Enter | Row greyed; bar stays open; nothing runs | `tst_search_bar::stubIsNotRunnable` |
| `sb_reveal` | Search `warp`, press Alt+Enter | Edit menu and Transform submenu open; Warp active | `tst_search_bar::altEnterRevealsMenu` |
| `sb_dismiss` | Open, press Escape; reopen, press outside; with nothing focused, Shift+Space, Escape, Shift+Space | Bar closes both times; it reopens after Escape and focus never lands in a text field | `tst_search_bar::dismissal`, `tst_search_bar::reopensAfterEscape` |
| `sb_look` | Open the bar over a document in `./build/pictura` | Centred, upper third, floating-panel styling | manual: visual styling and placement need a human eye |
