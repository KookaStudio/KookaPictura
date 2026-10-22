# Test plan — Shift+Space search bar

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `sb_open` | Show the window, focus the canvas host, press Shift+Space | Bar visible, empty query, query field focused | `tst_search_bar::shiftSpaceOpens` |
| `sb_hint` | Read the status hint; rebind `help.search` to Ctrl+K | Hint shows Shift+Space, then Ctrl+K | `tst_search_bar::rebindUpdatesHint` |
| `sb_textfield` | Focus a QLineEdit in the window, press Shift+Space | The field gains a space; bar stays closed | `tst_search_bar::textFieldKeepsShiftSpace` |
| `sb_typeedit` | Open a type edit on a document, press Shift+Space | The edit's text gains a space; bar stays closed | `tst_search_bar::typeEditKeepsShiftSpace` |
| `sb_space` | Send Space, release; send Shift+Space to the window's key handler | Space starts the pan; Shift+Space does not | `tst_search_bar::spaceStillPans` |
| `sb_sources` | Build the index from the window's registry | Command, tool, panel, preference and control entries present; each panel once; `help.search` absent | `tst_search_bar::indexCoversSources` |
| `sb_labels` | Clean `&File`, `R&&D`, `Fish & Chips`, `Liquify……`, `Save &As...<Tab>Ctrl+Shift+S` | `File`, `R&D`, `Fish & Chips`, `Liquify`, `Save As` | `tst_search_bar::cleanLabelsAndWhitespace` |
| `sb_controls` | Resolve every curated control in the window | Container and control widget found for each row | `tst_search_bar::curatedControlsExist` |
| `sb_warp` | Search `warp` | First hit is `Edit > Transform > Warp` | `tst_search_bar::warpFindsTransformWarp` |
| `sb_whitespace` | Search `transform<Tab>warp<Newline>` | First hit is `Edit > Transform > Warp` | `tst_search_bar::cleanLabelsAndWhitespace` |
| `sb_rank` | Search `transform warp`, `layers`, and a synthetic index | AND across words; exact > prefix > path-only; runnable first | `tst_search_bar::rankingOrder` |
| `sb_run` | Open a document, search `warp`, press Enter | Warp session starts; bar closes | `tst_search_bar::enterRuns` |
| `sb_run_targets` | Hide Layers, Enter on it; Enter on General; Enter on Blend Mode; Enter on Options bar > Brush > Flow | Layers shown and outlined; Preferences on General; Blend list open; Brush active and Flow field focused, text selected | `tst_search_bar::enterRunsPanelPreferenceControl` |
| `sb_stub` | Search a stub command, press Enter, click it, then Alt+Enter | Row greyed; bar stays open; nothing runs; Alt+Enter closes the bar and opens File > Export | `tst_search_bar::stubIsNotRunnable` |
| `sb_reveal` | Search `warp`, press Alt+Enter | Edit menu and Transform submenu open; Warp active | `tst_search_bar::altEnterRevealsMenu` |
| `sb_reveal_place` | Alt+Enter on Magic Wand, the Layers panel, and Blend Mode | Toolbox slot, panel, and combo outlined; tool unchanged; list closed | `tst_search_bar::altEnterOutlinesInPlace` |
| `sb_navigate` | Search `layer`, press Up, Down, Up, then Down past the end | Row 0, 0, 1, 0, then the last row | `tst_search_bar::upDownMovesAndClamps` |
| `sb_dismiss` | Open, press Escape; reopen, press outside; with nothing focused, Shift+Space, Escape, Shift+Space | Bar closes both times; it reopens after Escape and focus never lands in a text field | `tst_search_bar::dismissal`, `tst_search_bar::reopensAfterEscape` |
| `sb_focus` | Send a popup focus-out to the field; then focus another line edit | Bar stays open; then closes | `tst_search_bar::focusLossCloses` |
| `sb_look` | Open the bar over a document in `./build/pictura` | Centred, upper third, floating-panel styling | manual: visual styling and placement need a human eye |
