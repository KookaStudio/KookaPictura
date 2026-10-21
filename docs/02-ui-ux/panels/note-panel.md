# Notes Panel

- **Spec ID:** `PAN-024`
- **Status:** `Draft`
- **Parity tier:** `Core` — the Note tool and the Notes panel are in both CS6 editions. (The Count tool in the same tool group is `Extended-only`; see `03-tools/note-and-count.md`.)
- **New in CS6:** `No` — the Note tool and Notes panel are carried from CS5. The CS6 Help lists the Note tool in the navigation/notes/measuring tools gallery and its shortcut table with no functional change called out.
- **Depends on:** `03-tools/note-and-count.md` (`TOOL-018` Note tool — canonical note creation/editing semantics; this spec is the **panel UI**, not a duplicate), `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/undo-history.md` (`ARCH-009`), `01-architecture/qt6-ui-design.md` (`ARCH-003`), `10-workflow-io/file-info-and-metadata.md` (annotation carriage in output formats).

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. The CS6 Help PDF fetched for this pass documents the
> Note tool only at gallery/shortcut level; panel specifics are from secondary
> CS6-era sources and are attributed accordingly. Anything unverified is under
> Open questions.

## CS6 behavior

**Note tool.** The CS6 Help tool gallery describes the Note tool as attaching notes to an image. It is grouped in the toolbox with the Count tool under the Eyedropper slot and cycles with `I`.

**Notes panel.** The Notes panel (`Window > Notes`) is the text-editing surface for the selected note. Selecting the Note tool and clicking the canvas creates a note icon and makes the panel active; the note text is typed into the panel. The panel navigates notes in the active image with **back/forward arrows** and supports adding and deleting notes. Double-clicking a note icon with the Note tool reopens it for editing in the panel. *(These panel behaviors are from CS6-era secondary sources, consistent with `03-tools/note-and-count.md`; the fetched CS6 PDF did not include a dedicated Notes-panel section.)*

**Author and color.** The Note tool's options bar exposes **Author** and **Color**, stored with the note. *(secondary; mirrored in `TOOL-018`)*

**Visibility.** Notes are non-printing overlays. They are shown/hidden with `View > Show > Notes`, or collectively through `View > Extras` (which also toggles grids, guides, selection edges, target paths, slices, annotations, count, and layer borders). *(The `View > Extras` grouping is sourced to the CS6 PDF; the `View > Show > Notes` entry is from secondary CS6-era sources.)*

**Storage/format.** Annotated documents can be saved in **PSD, PDF, or TIFF**; notes are excluded from print and from the raster composite. *(`TOOL-018` cites the same formats; the CS6 PDF confirms `View > Extras` toggles "annotations".)*

**Audio Annotation.** An audio counterpart existed in earlier Photoshop versions; its CS6 status is not established by the fetched reference and is carried as an Open question in `03-tools/note-and-count.md`.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Toolbox (`I` group) | Note tool | `I` cycles / hold Eyedropper | Source: CS6 tool gallery/shortcut table |
| Canvas | Note icon overlay | click to place; double-click to edit | Non-printing anchor |
| Options bar (Note) | Author field | — | *(secondary)* |
| Options bar (Note) | Color swatch | — | *(secondary)* |
| `Window > Notes` | Menu → dock panel | — | Text editor + navigation |
| Notes panel | Text editor | — | Type/edit the selected note |
| Notes panel | Back / Forward | — | Cycle notes in the active image |
| Notes panel | Add / Delete note | — | Create/remove a note |
| `View > Show > Notes` | Menu | — | Show/hide note icons *(secondary)* |
| `View > Extras` | Menu | — | Toggles notes with other Extras (sourced) |
| `File > Save` / Save As | Menu | `Ctrl+S` / `Ctrl+Shift+S` | Carries notes in PSD/PDF/TIFF |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Note text | multiline string | empty | any | Panel editor |
| Author | string | system/empty *(not stated)* | — | Options bar; metadata on the note |
| Note color | color | default swatch *(not stated)* | color picker | Options bar icon color |
| Anchor | point | click position | document space | Canvas placement |
| Note index | int | 1 | 1..N in document order | Back/forward navigation |
| Visible | bool | on | on/off | `View > Show > Notes` / Extras |
| Note count | int | 0 | unlimited | Per document |

## Algorithms & pipeline

The panel is a **list/editor view over the document's note collection**; note creation and hit-testing belong to `TOOL-018`.

1. **Store** — each note is `{ id, anchor: point, text, author, color, created }` in document space, on the document model (`ARCH-008`), rendered as a non-printing overlay and never composited.
2. **Create** — Note-tool click appends a note (anchor = click point) and focuses the panel editor; empty notes may be discarded on deselect/commit *(inferred)*.
3. **Edit** — the panel edits the selected note's text; double-click on an icon selects that note. Edits are committed per edit session (keystroke-coalesced) as one undo command.
4. **Navigate** — back/forward walk the document's notes in order; the canvas scrolls to and highlights the selected note.
5. **Show/hide** — a view flag gates the overlay rendering; matches `View > Show > Notes` / `View > Extras`.
6. **Serialize** — notes are written to PSD/PDF/TIFF annotation data; JPEG/PNG and other formats cannot carry them and must warn/drop per `TOOL-018`. Exact PSD keys are unresolved.

## Rust module mapping

Consistent with `03-tools/note-and-count.md`:

- `pictura_core::annotations::Note` — `{ id: NoteId, anchor: Point, text: String, author: String, color: Color }`.
- `pictura_core::annotations::NoteStore` — CRUD, ordered iteration for back/forward, overlay culling, serializer hooks.
- `pictura_core::command::EditNote` — add/edit/delete as one undo command (`ARCH-009`).
- `pictura_ui_bridge::NoteSelection` — selected `NoteId` + visible flag for the panel.

Crossing types: `NoteId`, `Point`, `Color`, note text. No Qt types cross into the core.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `NotesPanel` | `QDockWidget` | Host; navigation, add/delete, visibility |
| `NoteTextEdit` | `QPlainTextEdit` | Multiline note text; commit on focus change/session end |
| `NoteAuthorColorControls` | `QLineEdit` + color button | Reflect the Note tool's options-bar author/color |
| `NoteNavigationBar` | `QToolBar` | Back/forward + note counter |
| `NoteOverlayItem` | `QGraphicsItem` | Canvas icon overlay; hit-test/double-click (`ARCH-003` QGraphicsView overlay) |

Widgets over QML for the dock and text editor (dense, keyboard-centric; `ARCH-003`). The icon overlay is a vector `QGraphicsItem` drawn above the GPU-composited canvas, not a widget.

## Data-model impact

- **Document data.** `Document` gains `notes: Vec<Note>`, serialized to PSD/PDF/TIFF annotation data so notes survive save/reopen; exact PSD keys are unresolved (`TOOL-018`).
- **Non-printing.** Notes are excluded from the raster composite, merge/flatten, and Save For Web; they never affect pixels.
- **Undo.** One history state per committed note add/edit/delete; record shape is a before/after list diff (`ARCH-009`).
- **Visibility is view state**, not document data, and is not serialized (whether CS6 persists it is unverified).
- **Format policy.** Formats that cannot carry notes (JPEG/PNG, etc.) must warn or drop explicitly, never silently destroy data.
- **Author/color** are note metadata, not global preferences.

## Edge cases

- **Empty note** — creating then deselecting without text should not leave a stray icon (behavior unverified in CS6).
- **Huge note count** — overlay rendering must cull off-screen icons; navigation must stay O(1)-ish.
- **Hidden notes** — `View > Show > Notes` off hides icons but must not delete or hide them from the panel list; whether the panel still lists them is unverified.
- **Format round-trip** — PSD/PDF/TIFF preserve notes; JPEG/PNG cannot; warn.
- **Print/merge/flatten/Save For Web** — notes must not appear in output.
- **Mode independence** — notes are color-model independent (8/16/32-bit, CMYK/Lab/Gray).
- **1-px/empty documents** — anchors may fall outside content bounds; clamp/allow gracefully.
- **PSB** — anchors and note counts must not overflow; preserve on round-trip.
- **GPU unavailable** — overlay renders on the CPU vector layer.
- **Undo/redo** — deleting a note and undoing restores text, author, color, and anchor exactly.
- **Audio Annotation** — if present in CS6, decide a representation; do not silently drop on open (Open questions).
- **Concurrent edit** — the panel edits one note at a time; switching notes commits/discards cleanly.

## Parity acceptance criteria

1. Given the Note tool with Author "A" and a color, clicking the canvas places a note icon and activates the Notes panel; text typed there is stored with the note.
2. Given a saved PSD/PDF/TIFF and a reopen, the note text, author, color, and anchor are restored.
3. Given several notes, the panel's back/forward arrows cycle them in document order and the canvas highlights the selected note.
4. Given `View > Show > Notes` off, note icons are hidden; on, they reappear; `View > Extras` toggles them with the other Extras.
5. Given a printed, merged, flattened, or Save-For-Web output, note icons do not appear in the raster.
6. Given a format that cannot carry notes, the app warns or refuses rather than silently discarding them.
7. Given a note edit, undo restores the previous text; given a note deletion, undo restores the note completely.
8. Given a CS6 PSD containing annotations, a `Core` build opens and re-saves it without losing the annotation data.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus (downloaded, text-extracted). Established: the Note tool gallery description (the tool attaches notes to an image); the `I`-group shortcut table entry for the Note tool; `View > Extras` shows/hides "annotations" among selection edges, grids, guides, target paths, slices, layer borders, and count. The fetched PDF contained **no dedicated Notes-panel or Note-tool procedure section**, which limits what is sourceable here.
- `https://www.bapugraphics.com/blog/adobe-photoshop-note-tool` — **(secondary, CS6-era)** Note tool procedure: options-bar Author and Color, click to place a note, Notes panel text entry, non-printing behavior, `View > Show > Notes` / `View > Extras`, double-click to edit, `Window > Notes` to cycle notes, notes saved in PSD/PDF/TIFF. Already cited by `03-tools/note-and-count.md`; re-used here for the panel UI.
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Notes_palette.html` — **(secondary, CS6-era)** Notes panel existence and the PSD/PDF/TIFF save claim (as cited in `03-tools/note-and-count.md`; snippet-level here).
- `https://searxng` query "Photoshop CS6 Notes panel Note tool author color Window Notes" — discovery only.

Consulted as search-result snippets only (not individually fetched; community/current-version):

- Adobe's current-version "notes" help pages surfaced by the query; later-version UI, not asserted as CS6.

Not used in this pass:

- `helpx.adobe.com` notes pages (HTTP 403 / current-version only).

## Open questions

- **Does CS6 ship a Notes panel procedure in the official Help?** The fetched PDF did not include one; the panel details rest on secondary CS6-era sources. *Resolves with:* the archived CS6 Help web page for "Notes panel" or a CS6 capture.
- **Audio Annotation CS6 status** (present, renamed, removed) and its representation. *Resolves with:* a CS6 UI capture; shared with `TOOL-018`.
- **PSD/PDF/TIFF annotation serialization keys** and whether notes are one annotation block or many. *Resolves with:* the file-format specs and a CS6-authored sample.
- **Whether note visibility persists** across sessions/documents. *Resolves with:* a CS6 test.
- **Empty-note and last-note deletion semantics** (kept, auto-discarded, confirm). *Resolves with:* a CS6 observation.
- **Exact default Author source and note color** on a fresh CS6 install. *Resolves with:* a preferences dump.
- **Is the Notes panel a single panel shared across documents or one per document** in CS6? *Resolves with:* a CS6 multi-document test.
