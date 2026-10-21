# Swatches and Libraries

- **Spec ID:** `CLR-003`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the Swatches panel and Preset Manager are unchanged in function. The HTML/CSS/SVG swatch import shown in the CS6 "What's New" list is explicitly **Creative Cloud only** and is not a CS6 Standard feature.
- **Depends on:** `CLR-001` color-models, `CLR-002` color-picker, `ARCH-007` color-management, `01-architecture/file-formats.md`, `02-ui-ux/panels/swatches-panel.md`, `10-workflow-io/presets-manager.md`, `10-workflow-io/save-and-save-as.md`, `07-color-painting/color-picker.md`.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

## CS6 behavior

The **Swatches panel** (`Window > Swatches`) stores colors used frequently; you can add or remove entries, or switch between libraries for different projects.

### Applying and displaying

- **Click** a swatch to set the **foreground** color.
- **`Ctrl`-click** (Windows) / **`Cmd`-click** (macOS) a swatch to set the **background** color.
- Display mode is changed "by choosing an option from the Swatches panel menu."

### Adding

1. Make the desired color the foreground color.
2. Either **click the New Swatch button** / choose **New Swatch** from the panel menu, or move the pointer over an empty slot in the bottom row of the panel (it becomes the Paint Bucket tool) and click to add the color, then name it and click OK.
3. Colors can also be added from the picker/dialog via **Add To Swatches**.

> New colors persist in the Photoshop preferences between editing sessions; saving a color to a library is what makes it permanent.

### Deleting

Drag a swatch to the **Delete** icon, or **`Alt`-click** / **`Option`-click** a swatch ("the pointer turns into scissors") and click. *(The Help's "scissors" cursor also applies in related contexts.)*

### Managing libraries (panel menu)

| Command | Behavior |
|---|---|
| **Load Swatches** | **Adds** a library to the current set; pick a file and click Load. |
| **Replace Swatches** | **Replaces** the current list with a different library; Photoshop offers to save the current swatches first. |
| **Name of a color library** (lower part of the menu) | Loads a specific color system (e.g. PANTONE/TOYO/etc.); the loaded library can replace or append to the current colors. |
| **Save Swatches** | Saves the current set as a library file; a library placed in `Presets/Swatches` appears by name at the bottom of the Swatches panel menu once the application restarts. |
| **Reset Swatches** | Restores the default swatch library; replace or append. |
| **Save Swatches For Exchange** | Saves a library for sharing across Photoshop, Illustrator, and InDesign. |

### Sharing between applications

Swatch libraries saved for exchange load into Photoshop, Illustrator, and InDesign, and colors match across those applications provided the color settings are synchronized. Swatches **excluded** from exchange (Help):

- From Illustrator/InDesign: **patterns, gradients, and the Registration swatch**.
- From Photoshop: **book color references, HSB, XYZ, duotone, monitorRGB, opacity, total ink, and webRGB** swatches.

### Preset Manager

`Edit > Presets > Preset Manager` (CS6 path; CS5 was `Edit > Preset Manager`) manages libraries of brushes, **swatches**, gradients, styles, patterns, contours, custom shapes, and preset tools. For swatches it supports:

- Display modes: **Text Only, Small/Large Thumbnail, Small/Large List**.
- **Load / Append / Replace**, rename, delete, **Save Set**, and **Reset**.
- Each library type has its own file extension and default folder; shipped presets live in the application's `Presets` folder, user libraries in the per-user Adobe presets location.

### Color libraries / spot books

The **named color-system entries at the bottom of the Swatches panel menu** are the same spot-color systems the picker exposes (`CLR-002`): PANTONE, TOYO Color Finder 1050, TRUMATCH, FOCOLTONE, HKS, DIC, ANPA-COLOR. Adobe ships several standard color libraries that can be loaded from the Swatches panel menu.

### Adobe Color Themes / Kuler (CS6 vs CC)

CS6 shipped **no built-in "Adobe Color Themes" panel**. A secondary source states that CC exposes the panel through `Window > Extensions > Adobe Color Themes`, while CS6 and earlier users can instead use the Color Themes web app at color.adobe.com. The CS6 Help likewise lists **Kuler** only as a *tablet (Adobe Touch) app* and never as an in-application panel. For CS6 parity the built-in mechanism is the Swatches panel + Preset Manager; any Color-Themes/Kuler integration is out of scope and belongs in `00-overview/feasibility-and-non-goals.md`.

### File formats

The CS6 Help names the operations (**Load/Replace/Save Swatches**, **Save Swatches For Exchange**) but not the file extensions. Secondary sources identify:

- **`.aco`** — "Adobe Color" binary, Photoshop's own single-palette swatch file.
- **`.ase`** — "Adobe Swatch Exchange", Adobe's cross-application palette format (RGB/CMYK/Lab/Gray entries plus names); **Save Swatches For Exchange** produces this format, and both `.aco` and `.ase` can be loaded.

These extensions are *(community-sourced; verify against CS6 capture)*.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Swatches` | Panel | — | Opens the Swatches panel |
| Swatches panel grid | Click | — | Sets foreground color |
| Swatches panel grid | `Ctrl`/`Cmd`-click | — | Sets background color |
| Swatches panel — New Swatch | Button / menu | — | Adds foreground color |
| Swatches panel — bottom empty row | Click | — | Paint-bucket add with name prompt |
| Swatches panel — Delete icon | Drag / `Alt`-click | — | Removes a swatch |
| Swatches panel menu | Menu | — | Load / Replace / Save / Reset / Exchange / book entries |
| Preset Manager (Swatches type) | Dialog | — | Load/Append/Replace, rename, delete, Save Set, Reset |
| Adobe Color Picker — Add To Swatches | Button | — | Adds to the panel (`CLR-002`) |
| HDR Color Picker — Add To Swatches | Button | — | Adds HDR color (`CLR-002`) |
| Panel menu — color-system entries | Menu | — | Spot books (PANTONE/TOYO/…) |
| Keyboard: Keys for the Swatches panel | Table | — | Create / set-background / delete |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Swatch color | color value | — | RGB / CMYK / Lab / HSB / book / grayscale | Stored per `CLR-001` model; spot swatches carry a book reference |
| Swatch name | string | generated | user text | Prompted on add/new-swatch |
| Display mode | enum | small thumbnails *(inferred)* | text / small/large thumbnail / small/large list | Panel- and Preset-Manager-level |
| Load / Replace / Append | action | — | file chooser | Append vs replace semantics |
| Default library | bundled set | Photoshop defaults | replace / append | Via **Reset Swatches** |
| Exchange filter | rule set | — | excludes patterns/gradients/Registration, book/HSB/XYZ/duotone/monitorRGB/opacity/total-ink/webRGB | Photoshop→CS apps |
| Preset folder | path | `Presets/Swatches` | any | Placed here → appears in menu after restart |

## Algorithms & pipeline

The panel is a **list model + persistence layer**; there are no image algorithms.

### Data layout

A swatch library is an ordered list of entries. Each entry is one of:

- **Process color** — a value in an RGB/CMYK/Lab/Gray model (`CLR-001`).
- **Spot / book color** — a reference `(book_id, ink_number, name, Lab value)`; the Lab value is the canonical display value (Illustrator/InDesign default to Lab for predefined spot colors; swapping to CMYK equivalents is a cross-app option outside CS6 Photoshop's scope).
- **Special** — `Registration` and the CC-only web/monitorRGB/opacity/total-ink/HSB/XYZ variants, which are filtered from exchange.

### Operations

- **Load** = `current.extend(library)`; **Replace** = `save_prompt(); current = library`; **Append-vs-replace** from the book entries follows the same rule.
- **Save** = serialize the current list; **Save For Exchange** = serialize after applying the exclusion filter.
- **Reset** = replace/append the bundled default library.
- **Preference persistence** = the panel's transient added colors are written to the Photoshop preferences store; a library file is the durable artifact.

### Format codecs

- `.aco` — binary header (version), count, then RGB triples; version 2 adds names and supports multiple color models *(community-sourced structure — treat as a behavioral target, not a byte contract)*.
- `.ase` — "Adobe Swatch Exchange": blocks with type/length, a global color count, and per-entry `(name, model, values)`; supports multiple palettes in one file *(community-sourced)*.
- The independent-creation implementation may choose its own internal format but must round-trip the two exchange formats for cross-application parity.

## Rust module mapping

- `pictura_presets::swatches` — `SwatchLibrary { entries: Vec<Swatch> }`, `Swatch { name, value: SwatchValue }`, `SwatchValue::Process(ColorValue) | Spot(SpotRef) | Special(SpecialKind)`.
- `pictura_presets::swatches::ops` — `load`, `replace`, `append`, `save`, `save_for_exchange`, `reset`, with the exclusion filter as a predicate.
- `pictura_presets::codec::aco` and `::ase` — encode/decode the two exchange formats (behind `pictura_presets::codec`).
- `pictura_color::library` — shared spot-book registry used by the picker (`CLR-002`) and the panel.
- `pictura_core::prefs` — persisted "session swatches" and panel display mode.
- `pictura_core::command` — panel mutations are **not** document History states; they are preference/preset mutations.

Crossing types: `Swatch`, `SwatchValue`, `SwatchLibrary`, `SpotRef`, `SpecialKind`, `ColorValue`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `SwatchesPanel` | `QDockWidget` | Panel host; toolbar (new/delete), grid, fly-out menu |
| `SwatchGridModel` | `QAbstractItemModel` | Ordered swatch list; name/role data; mutation API |
| `SwatchGridDelegate` | `QStyledItemDelegate` | Paints swatch cells; handles click / `Ctrl`-click semantics |
| `SwatchGridView` | `QListView` (icon mode) | Cell layout and bottom-row empty-cell add affordance |
| `SwatchLibraryMenu` | `QMenu` | Load / Replace / Save / Reset / Exchange / book entries |
| `NewSwatchDialog` | `QDialog` | Name entry on add |
| `PresetManagerDialog` | `QDialog` | Preset Type = Swatches; load/append/replace, rename, delete, Save Set, Reset |
| `ColorLibraryDialog` | shared with `CLR-002` | Book browser |

Widgets, not QML: dense docked grid with keyboard/menu-driven management, consistent with `ARCH-003`.

## Data-model impact

- **Not document data.** Swatch libraries live in preset files and the preferences store; a PSD does not embed the panel contents *(inferred)*.
- **Spot swatches** chosen for painting fill/spot channels interact with the document via channel operations (`02-ui-ux/panels/channels-panel.md`), not by adding document fields here.
- **Preference record:** `{ display_mode, session_swatches: Vec<Swatch> }`.
- **Preset library record:** a serialized `SwatchLibrary` plus its file path; loaded libraries may be session-only until saved.
- **Undo:** panel add/delete/load are not document History operations (matches CS6; the History panel tracks image edits only).
- **Serialization:** `.aco` / `.ase` for exchange; the internal library representation is a project decision (`10-workflow-io/presets-manager.md`).

## Edge cases

- **8/16/32-bit** — swatches are model colors; a 32-bpc HDR color added from the HDR picker must retain float precision if the library format can carry it, else warn on lossy save.
- **CMYK/Lab/spot** — loading a CMYK-only library into a non-CMYK document must still display via the working space; spot book colors default to Lab.
- **Duplicate colors** — the CC HTML/CSS/SVG import de-duplicates ("only one instance of the color is added"); the CS6 panel does not document de-duplication, so adding the same color twice is allowed *(inferred)*.
- **Missing/corrupt library file** — fail with a clear message; do not partially load.
- **Very large libraries** — grid virtualization; loading must not block the UI.
- **Exchange exclusions** — a library containing excluded kinds must filter them silently on exchange save, matching CS6.
- **Reset** with unsaved session swatches — offer replace/append and allow cancelling.
- **Preset folder discovery** — a library dropped into `Presets/Swatches` appears in the menu only after restart; the app should mirror that or document a refresh.
- **Concurrent edits** — external modification of a library file between load and save.
- **Undo/redo** — deleting a swatch has no History entry; provide a local undo affordance or accept CS6's lack of one *(inferred; verify)*.
- **Linux paths** — preset discovery under `~/.config`/`~/.local/share` per packaging (`01-architecture/build-and-packaging.md`).

## Parity acceptance criteria

1. Given the default panel, clicking a swatch sets the foreground color and `Ctrl`/`Cmd`-click sets the background color.
2. Given a foreground color, clicking the New Swatch button prompts for a name and appends a swatch.
3. Given a new swatch added without saving a library, it is present after an application restart (preference persistence).
4. Given library A active and library B loaded with **Load**, A+B are both present; with **Replace**, only B remains and the user is offered a save-first prompt.
5. Given **Reset Swatches**, the default library is restored (replace or append as chosen).
6. Given a library saved into `Presets/Swatches` and an application restart, its name appears at the bottom of the Swatches panel menu.
7. Given **Save Swatches For Exchange** on a library containing patterns/gradients/book/HSB/XYZ/duotone/monitorRGB/opacity/total-ink/webRGB entries, those entries are omitted from the saved file.
8. Given a `.aco` and a `.ase` library, each loads via the panel menu and the swatch colors match their source values within one 8-bit step.
9. Given a spot-color swatch accepted from a book, the panel row and the picker's Color Libraries selection refer to the same book/ink entry.
10. Given Preset Manager → Swatches, Load/Append/Replace, rename, delete, Save Set, and Reset all behave as described and persist across restart.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — official CS6 Help reference. Established: the Swatches panel purpose and `Window > Swatches`; click = foreground / `Ctrl`/`Cmd`-click = background; add via New Swatch / panel-menu New Swatch / empty-bottom-row paint-bucket add with name prompt; preference persistence of new colors vs library durability; delete via trash drag or `Alt`/`Option`-click scissors; Load (add) vs Replace (replace, with save-first option) vs named-book entries (replace or append); Save Swatches and the `Presets/Swatches` menu-after-restart rule; Reset Swatches (replace/append); Save Swatches For Exchange and the full exclusion list; the cross-application sharing promise; the lower-part-of-menu color systems; the standard color libraries shipped by Adobe and their loading from the Swatches panel menu; Preset Manager paths (CS5 vs CS6) and swatch operations; the rule that each library type has its own file extension and default folder.
- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — same file, "What's New in CS6": "Read swatches from HTML, CSS, and SVG files" is listed under Creative Cloud-only features; the CS6 JDI list contains no Swatches-panel change.
- `https://colorslurp.com/blog/a-complete-guide-to-ase-files` — secondary/community: `.ase` = "Adobe Swatch Exchange" storing multiple named palettes and sharing across Illustrator/Photoshop/InDesign; `.aco` is Photoshop's single-palette format; Photoshop imports via the Swatches panel menu and exports via **Save Swatches for Exchange**.
- `https://docs.fileformat.com/settings/aco/` — secondary/community: ACO described as a binary Photoshop/Adobe color-swatch file (page content largely navigational; used only to confirm the extension's purpose).
- `https://photoshoptrainingchannel.com/tips/adobe-color-themes` — secondary/community: states that CC reaches the in-app panel through `Window > Extensions > Adobe Color Themes`, while CS6 and earlier rely on the Color Themes web app. Establishes that the in-app Color Themes panel is a CC feature, not CS6.

Not fetched (HTTP 403 from this environment): `helpx.adobe.com` "Share swatches between applications" and "Add swatches from HTML CSS and SVG" pages linked from the CS6 PDF.

## Open questions

- **`.aco` / `.ase` extensions** are not named by the CS6 Help; the mapping is community-sourced. *Resolves with:* a CS6 save/load capture and file inspection.
- **Exact default display mode** of the Swatches panel (Small Thumbnail assumed). *Resolves with:* a CS6 first-run capture.
- **De-duplication behavior** of the CS6 panel (only the CC HTML/CSS/SVG path documents de-dup). *Resolves with:* a CS6 add-twice test.
- **Whether panel add/delete is undoable** in CS6. *Resolves with:* a CS6 experiment.
- **Book library provisioning and licensing** (bundling PANTONE/TOYO/DIC data). *Resolves with:* `00-overview/licensing-and-provenance.md`.
- **Kuler in CS6** — whether a stock CS6 install exposed Kuler via `Window > Extensions`, and whether Kooka Pictura should stub it. *Resolves with:* a CS6 installation capture / Adobe Exchange archive.
- **Internal library serialization** format (ACO/ASE vs a project format) and its 16/32-bit fidelity. *Resolves with:* `10-workflow-io/presets-manager.md` and `01-architecture/file-formats.md`.
