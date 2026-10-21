# Swatches Panel

- **Spec ID:** `PAN-011`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the Swatches panel and Preset Manager are unchanged. The HTML/CSS/SVG swatch import in the CS6 "What's New" list is **Creative Cloud only**.
- **Depends on:** `CLR-001` color-models, `CLR-002` color-picker, `CLR-003` swatches-and-libraries, `ARCH-003` qt6-ui-design, `ARCH-007` color-management, `10-workflow-io/presets-manager.md`, `02-ui-ux/preferences.md`.

> This document owns the **Swatches panel UI surface**. The library data model,
> `.aco`/`.ase` codecs, spot-book registry, and load/replace/append semantics are
> specified in `CLR-003`; the Preset Manager dialog is shared with
> `10-workflow-io/presets-manager.md`. All crate, module, widget, and type names
> are **design proposals**. The repository contains a working implementation; this
> file stays the behavioural contract for the feature, and
> `docs/dev/doc-code-parity-gaps.md` records what shipped and what did not. Facts
> not confirmed by a fetched CS6 source are marked *(inferred)*.

## CS6 behavior

`Window > Swatches` opens the **Swatches panel**, which stores colors you use often. Colors can be added to or deleted from the panel, and different color libraries can be displayed for different projects.

**Default placement (CS6 Essentials workspace).** The panel is tabbed behind the **Color** panel at the **top of the right-hand main column**; Color is the default-active tab. (Source: Photoshop Essentials, *Managing Panels In Photoshop CS6*.)

### Applying

- **Click** a swatch → sets the **foreground** color.
- **`Ctrl`-click** (Windows) / **`Cmd`-click** (macOS) → sets the **background** color.

Swatches are laid out as a **grid of colored cells**, expanding to the panel width; display style is changed from the panel menu (`CLR-003`). The CS6-era default view is the thumbnail grid (a list/text view is available from the panel menu).

### Adding

1. Make the desired color the foreground color.
2. Either click the **New Swatch** button / choose **New Swatch** from the panel menu, or position the pointer over an empty space in the bottom row of the Swatches panel — the pointer becomes the **Paint Bucket** tool — and click to add the color, then enter a name and click OK.
3. Colors can also be added from the picker or HDR picker via **Add To Swatches** (`CLR-002`).

> New colors are stored in the Photoshop preferences file and persist between editing sessions. To save a color permanently, save it in a library.

### Deleting and editing

- Drag a swatch onto the **Delete** icon, or **`Alt`-click** / **`Option`-click** a swatch ("the pointer turns into scissors") and click.
- Double-click a swatch to **rename** it *(inferred; the Help documents renaming in the Preset Manager)*.

### Managing libraries

The panel menu drives library operations; full semantics live in `CLR-003`.

| Command | Panel behavior |
|---|---|
| **Load Swatches** | File chooser; appends a library to the current set |
| **Replace Swatches** | File chooser; replaces the current set, offering a save-first prompt |
| **Save Swatches** | Writes the current set; placing it in `Presets/Swatches` lists its name after restart |
| **Reset Swatches** | Restores the default library (replace or append) |
| **Save Swatches For Exchange** | Writes the filtered cross-application library |
| **Named color-system entries** (bottom of menu) | Load a spot book (PANTONE/TOYO/…); replace or append |
| **Display options** | Text Only / Small / Large Thumbnail / Small / Large List |

### Preset Manager integration

`Edit > Presets > Preset Manager` (CS6 path) manages swatches alongside brushes, gradients, styles, patterns, contours, custom shapes, and tool presets: display modes, **Load / Append / Replace**, rename, delete, **Save Set**, and **Reset**. Libraries placed in the default preset folder appear in the panel menu after restart.

### Color libraries (spot books)

The named entries at the bottom of the panel menu are the same spot-color systems the picker exposes (`CLR-002`): PANTONE, TOYO Color Finder 1050, TRUMATCH, FOCOLTONE, HKS, DIC, ANPA-COLOR. The panel is a *view* over the shared `pictura_color::library` registry.

### Not in CS6

There is **no** in-application Adobe Color Themes/Kuler panel and **no** built-in HTML/CSS/SVG import in CS6; both are CC-era (`CLR-003`, `OVR-003`).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Swatches` | Panel | — | Opens the Swatches panel |
| Swatch grid cell | Click | — | Sets foreground color |
| Swatch grid cell | `Ctrl`/`Cmd`-click | — | Sets background color |
| Swatch grid cell | Double-click | — | Rename *(inferred)* |
| Swatch grid cell | `Alt`/`Option`-click | — | Delete (scissors cursor) |
| Swatch grid cell | Drag → Delete icon | — | Delete |
| New Swatch button | Button | — | Adds the foreground color |
| Bottom empty row | Click | — | Paint-bucket add + name prompt |
| Delete icon | Button | — | Drop target for swatch deletion |
| Panel menu | Menu | — | Display options; Load/Replace/Save/Reset/Exchange; book entries |
| Adobe Color Picker → Add To Swatches | Button | — | Adds via `CLR-002` |
| HDR Color Picker → Add To Swatches | Button | — | Adds via `CLR-002` |
| Preset Manager (Swatches type) | Dialog | `Edit > Presets > Preset Manager` | Load/Append/Replace, rename, delete, Save Set, Reset |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Display mode | enum | Thumbnail (small) | Text Only / Small Thumbnail / Large Thumbnail / Small List / Large List | Panel menu and Preset Manager (CS6 Help lists the five modes in the Preset Manager; the panel menu uses the same set). |
| Grid columns | int (derived) | auto-fit | 1 … panel width | Cell size follows display mode |
| Swatch name | string | generated | user text | Prompted on add; rename in Preset Manager |
| Active swatch | index | — | 0 … n−1 | Single-select for delete/rename |
| Library set | list | bundled defaults | any loaded library | Load appends; Replace swaps |
| Delete drop target | target | — | — | Trash icon |
| Spot-book entry | menu item | — | PANTONE / TOYO / TRUMATCH / FOCOLTONE / HKS / DIC / ANPA | Shared with `CLR-002` |

The swatch **value** ranges are `CLR-001`'s per-model ranges; spot swatches carry `(book_id, ink_number, name, Lab)`.

## Algorithms & pipeline

The panel contains no image algorithms. Its work is grid presentation and command routing, both delegated:

- **Grid layout** — wrap the ordered `SwatchLibrary` entries into rows at the current cell size; the bottom row exposes a trailing empty "add" cell.
- **Hit-testing** — map a cell click to `{ set foreground | set background | delete | rename }` from the modifier state.
- **Command routing** — add/delete/load/replace/save call `pictura_presets::swatches::ops` (`CLR-003`); the panel only supplies the active color and a name.
- **Thumbnail painting** — fill each cell with the swatch's display value, converting through the working space when the swatch model differs from the document (`ARCH-007`).
- **Preset Manager** — the same model is re-hosted in a multi-type dialog; the panel and manager must not diverge.

## Rust module mapping

- `pictura_presets::swatches` — `SwatchLibrary`, `Swatch`, `SwatchValue`, and the `ops` module (`CLR-003`); authoritative, Qt-free.
- `pictura_presets::swatches::panel` — `SwatchesPanelModel { entries, display_mode, selected, trailing_add_cell }`, `cell_rects()`, `hit_test(point)`, and mutation intents.
- `pictura_color::library` — shared spot-book registry (`CLR-002`/`CLR-003`).
- `pictura_core::command` — `AddSwatch` / `DeleteSwatch` routed from the panel (no document History).
- `pictura_core::prefs` — `swatches_display_mode`, session swatches (`CLR-003`).

Crossing types: `Swatch`, `SwatchValue`, `SwatchLibrary`, `DisplayMode`, `ColorValue`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `SwatchesPanel` | `QDockWidget` | Host; toolbar (New/Delete), grid view, fly-out menu |
| `SwatchGridModel` | `QAbstractItemModel` | Ordered entries, name/color roles, trailing add cell, mutation API |
| `SwatchGridDelegate` | `QStyledItemDelegate` | Paints cells and the add cell; scissors/paint-bucket cursor states |
| `SwatchGridView` | `QListView` (IconMode) | Wrapping grid; click/`Ctrl`-click/`Alt`-click; drag-to-delete |
| `SwatchesToolbar` | `QToolBar` | New Swatch, Delete drop target |
| `SwatchLibraryMenu` | `QMenu` | Display options; Load/Replace/Save/Reset/Exchange; book entries |
| `NewSwatchDialog` | `QDialog` | Name entry on add/rename |
| `PresetManagerDialog` | `QDialog` | Shared multi-type manager (`10-workflow-io/presets-manager.md`) |
| `SwatchesController` | `QObject` | Bridges grid intents to `pictura_presets::swatches::ops`; emits fg/bg changes |

Widgets, not QML: dense docked grid with menu/keyboard management, consistent with `ARCH-003`. The `QListView` + custom delegate (rather than a hand-rolled paint widget) gives keyboard navigation and drag-and-drop for free.

## Data-model impact

- **Not document data.** Swatch libraries live in preset files and the preferences store; a PSD does not embed the panel contents *(inferred)* (`CLR-003`).
- **Preference record:** `{ display_mode, session_swatches }`.
- **Preset library record:** a serialized `SwatchLibrary` plus its path; loaded libraries are session-only until saved.
- **Undo:** panel add/delete/load are **not** document History operations. CS6 documents no panel-level undo; a local undo affordance is an explicit non-parity option (Open questions).
- **Selection** is UI state, never serialized.
- **Spot swatches** applied to the document do so through channel operations, not new document fields.

## Edge cases

- **Empty / single-swatch library** — grid must render cleanly and the add cell must remain reachable.
- **Very large libraries** — virtualize rows; loading must not block the UI thread.
- **Missing/corrupt library file** — fail with a clear message; never partially load (`CLR-003`).
- **CMYK/Lab/spot swatches in an RGB document** — cells display via the working space; no false gamut errors.
- **32-bpc HDR swatches** — a float color added from the HDR picker must retain precision or warn on lossy library save (`CLR-003`).
- **Exchange exclusions** — the panel must not appear to keep excluded kinds after **Save Swatches For Exchange**.
- **Reset / Replace with unsaved session swatches** — prompt before discarding.
- **`Presets/Swatches` discovery** — a library dropped in the folder appears only after restart; mirror or document a refresh (`CLR-003`).
- **Duplicate colors** — CS6 does not document de-duplication, so adding twice is allowed *(inferred)*.
- **High-contrast / dark theme** — cell borders and the add-cell glyph must stay visible on dark swatches.
- **Keyboard-only** — grid navigation, add, delete, and rename must be reachable without a pointer.
- **Linux preset paths** — discovery under `~/.config` / `~/.local/share` (`ARCH-004` build-and-packaging).

## Parity acceptance criteria

1. Given the default panel, clicking a swatch sets the foreground color and `Ctrl`/`Cmd`-click sets the background color.
2. Given the pointer over the bottom empty row, the cursor becomes the Paint Bucket, and clicking adds a named swatch.
3. Given a new swatch added without saving a library, it is still present after an application restart.
4. Given library A active, **Load** library B yields A+B; **Replace** with B leaves only B and offers the save-first prompt.
5. Given `Alt`/`Option`-click on a swatch, the swatch is deleted and the scissors cursor is shown.
6. Given **Reset Swatches**, the default library is restored (replace or append as chosen).
7. Given a library saved into `Presets/Swatches` and a restart, its name appears at the bottom of the panel menu.
8. Given a `.aco` and a `.ase` library, both load from the menu and their colors match the source within one 8-bit step (`CLR-003`).
9. Given Preset Manager → Swatches, Load/Append/Replace, rename, delete, Save Set, and Reset all behave as described and persist across restart.
10. Given the panel at any display mode, every swatch is reachable and the active selection survives a mode change.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — official CS6 Help reference (downloaded and text-extracted). Established: `Window > Swatches` and the panel purpose; click = foreground and `Ctrl`/`Cmd`-click = background; add via New Swatch / panel-menu New Swatch / bottom-empty-row paint-bucket add with a name prompt; the preference-persistence caveat for new colors; delete via trash drag or `Alt`/`Option`-click scissors; the panel menu display options and the Preset Manager's five display modes (Text Only / Small Thumbnail / Large Thumbnail / Small List / Large List); the "Keys for the Swatches panel" table (empty-area create, `Control`/`Command`-click background, `Alt`/`Option`-click delete); the lower-part named color-system entries; the cross-application requirement that color settings be synchronized; the CS6 **Add swatches from HTML CSS and SVG** page being marked Creative Cloud; `Edit > Presets > Preset Manager` (CS6 path). Library file formats and Preset Manager operations are detailed in `CLR-003`.
- `https://docs.merkulov.design/choose-colors-in-the-color-and-swatches-panels` — secondary mirror: corroborates the Swatches panel purpose and the newer-version extensions (groups, legacy swatches) that are **not** CS6 and are therefore out of scope.
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Swatches_palette.html` (fetched) — CS6-era book companion: the default Swatches panel view is the thumbnail grid, with an alternative list view from the fly-out menu; add via the empty bottom area; `Alt`-click to erase; library load/replace; swatch reordering via Preset Manager.
- `https://www.photoshopessentials.com/basics/managing-panels-in-photoshop-cs6` (fetched) — CS6 Essentials default workspace: Swatches is tabbed with Color at the top of the main column.

Not used in this pass:

- `helpx.adobe.com` "Share swatches between applications" and swatch-import pages (HTTP 403 from this environment); the archived CS6 PDF and `CLR-003` were used instead.

## Open questions

- **Rename affordance in the panel** — whether double-click renames in place or opens a dialog, versus only in Preset Manager. *Resolves with:* a CS6 panel capture.
- **Panel add/delete undo** — CS6 documents none; whether to add a local undo is a product decision (non-parity). *Resolves with:* the panel UX decision.
- **De-duplication on add** — CS6 does not document it (only the CC HTML/CSS/SVG path de-dups). *Resolves with:* a CS6 add-twice test.
- **Grid cell metrics and wrapping threshold** per display mode. *Resolves with:* a CS6 capture at each mode.
- **Spot-book provisioning and licensing** — whether PANTONE/TOYO/DIC data can be bundled (`OVR-004` licensing-and-provenance). *Resolves with:* the legal review.
- **Preset Manager parity scope** — the dialog is shared across preset types; its full spec lives in `10-workflow-io/presets-manager.md` (planned). *Resolves with:* that spec.
