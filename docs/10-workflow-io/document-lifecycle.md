# Document Lifecycle

- **Spec ID:** `WF-001`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — CS6 adds **Save in Background** (keep working while a Save completes) and **Automatically Save Recovery Information** (crash auto-recovery, default 10-minute interval). Both live on the File Handling preferences page. CS5 had neither in this form. See `## Open questions` on the exact preference coupling.
- **Depends on:** `ARCH-008` document-model, `ARCH-011` file-formats, `ARCH-007` color-management, `ARCH-009` undo-history, `02-ui-ux/preferences.md`, `11-cross-cutting/crash-recovery-and-autosave.md`, `WF-002` open-and-new, `WF-003` save-and-save-as, `WF-005` web-export-and-slices.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. CS6 behavior is taken from the fetched CS6 Help PDF
> unless marked *(inferred)*. Where the Help text is silent (for example the exact
> dirty-state indicator), that is called out under `## Open questions`.

## CS6 behavior

A **document** is opened or created once, has one in-memory identity, an optional
backing file, a dirty/clean state, and one or more window views. The lifecycle
commands group under `File`.

### Create and duplicate

- `File > New` opens the **New** dialog and, on OK, opens an untitled document
  window in memory (never written to disk until a Save). See `WF-002`.
- `Image > Duplicate` copies the whole image (all layers, layer masks, and
  channels) into available memory without saving to disk. The **Duplicate Image**
  dialog takes a name and a **Duplicate Merged Layers Only** checkbox (off by
  default, preserving layers). Unlike `Save As`, Duplicate is explicitly the
  in-memory copy path; `File > Save As` with **As A Copy** is the on-disk copy
  path (`WF-003`).

### Open

`File > Open`, `File > Open Recent`, `File > Open As`, and
`File > Open As Smart Object` are covered in `WF-002`. A camera raw or PDF file
opens through a format-specific options dialog before the document materialises
in Photoshop.

Pictura opens off the GUI thread and inserts the document tab immediately. The
tab is titled with the file name and shows an empty workspace while the read,
decode, first composite, and history capture run; its close button is an
animated spinner that cancels (discards the result) and middle-click also
cancels. Another active document stays active and visible until the open
succeeds. A generic PDF's probe and Import PDF dialog show no loading tab; the
tab appears when a chosen page or image render starts. The lower-left
`Opening <file>…` banner is gone.

While a tab loads, its empty workspace shows a centered progress card with the
file name, the current phase, a progress bar, the percentage, and Cancel. The
phases are `Reading file`, `Decoding`, `Compositing`, and `Preparing layers`,
plus `Rendering page`, `Rendering image`, or `Rendering EPS` for imports. The
percentages are phase weights, not measured work: a long decode holds its phase
percentage while the tab spinner and phase text show that work continues. The
card's Cancel behaves like the tab spinner: it removes the tab, discards the
worker result, restores the previous tab, and shows no error.

### Save

- `File > Save` writes changes to the current file **in the current format**. If
  the document has never been saved (no path, no format) Save falls through to
  Save As behavior.
- `File > Save As` writes to a different name, location, or format; format
  availability and per-format options are in `WF-003`.
- CS6 **Save in Background** lets you keep editing after choosing a Save command
  instead of waiting for the write to finish. Help warns that if you regularly
  save large files and want the most consistent performance, disable
  **Save In Background** in the File Handling preferences.

### Close, revert, and quit

- `File > Close` closes the active document window; `File > Close All` closes
  every open document *(inferred — the CS6 Help PDF does not document the Close
  All item; present in the shipped File menu)*.
- Unsaved changes prompt a **Save / Don't Save / Cancel** confirmation before a
  document is discarded *(inferred from standard behavior; the Help PDF does not
  describe the prompt wording)*.
- On Windows, `Shift+Ctrl+W` closes the file in Photoshop and opens Bridge;
  macOS uses `Shift+Command+W`. This is the only Close-family shortcut the Help
  shortcut tables list.
- `File > Revert` (`F12`) discards unsaved changes and reloads the last saved
  version. Help notes Revert is added as a History state and can itself be undone.

### Dirty state

Photoshop marks a window as having unsaved changes in the title bar; the exact
glyph (a trailing `*`, an asterisk, or platform convention) is not stated in the
Help PDF and is treated as *(inferred)*. Dirty state resets on a successful Save
or Revert. A document becomes dirty on any undoable history change; history
granularity is defined by `ARCH-009`.

### Compatibility (Maximize PSD and PSB compatibility)

- `Edit > Preferences > File Handling` (Windows) or
  `Photoshop > Preferences > File Handling` (macOS) exposes **Maximize PSD and PSB
  File Compatibility** with three settings:
  - **Always** — write a composite (flattened) image alongside the layer data.
  - **Ask** — prompt at each save.
  - **Never** — write a layered image only.
- The composite makes layered PSD/PSB readable by older Photoshop versions and by
  applications that do not understand layers, loads faster elsewhere, and can be
  required for readability. `Ask`/`Never` significantly reduce file size.
- Saving in an **earlier Photoshop version** discards features that version does
  not support (Help's note under the compatibility topic). The version target is
  chosen by the Save As format list and, for PSD, by the compatibility preference
  rather than by a numeric version picker in CS6 *(inferred)*.

### Auto-recovery (CS6)

- Auto-recovery stores crash-recovery information at a specified interval; the
  **default is ten minutes**. If the application crashes, the work is recovered
  the next time Photoshop starts.
- The interval is set under File Handling: Help's wording is to select **Save In
  Background** and then select an interval from the **Automatically Save Recovery
  Information** menu. (Whether recovery is independent of background-save is
  unresolved; see `## Open questions`.)

### Document status

Two surfaces report status:

- The **document window status bar** shows current magnification and file size,
  brief instructions for the active tool, and (clicked) document width, height,
  channel count, and resolution. `Ctrl`/`Command`-clicking shows tile width and
  height. It can also show **Document Sizes**, **Document Profile**,
  **Document Dimensions**, **Measurement Scale**, **Scratch Sizes**,
  **Efficiency**, **Timing**, **Current Tool**, and (32-bit HDR only) a **32-bit
  Exposure** preview slider.
- **Document Sizes** has two numbers: the left is the approximate saved,
  flattened PSD size (printing size); the right is the approximate size
  including layers and channels.

### Recent files

`File > Open Recent` lists recently used files. The count is controlled by
**Recent File List Contains** in File Handling (`WF-002`).

### Language / edition notes

Duplicate, Save, Save As, Close, Revert, and the status bar are edition-neutral
(Standard and Extended). Camera Raw and video open paths add Extended-only
entries (`WF-002`, `WF-004`).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `File > New` | Dialog + command | `Ctrl/Cmd+N` *(inferred)* | Creates untitled document (`WF-002`) |
| `Image > Duplicate` | Dialog | — | In-memory copy, optional merged-layers-only |
| `File > Open` | Dialog | `Ctrl/Cmd+O` | `WF-002` |
| `File > Open Recent` | Submenu | — | Count from File Handling preference |
| `File > Open As` | Dialog | — | Windows; force a file type |
| `File > Open As Smart Object` | Dialog | — | Single-layer smart-object document |
| `File > Save` | Command | `Ctrl/Cmd+S` *(inferred)* | Current format |
| `File > Save As` | Dialog | `Ctrl/Cmd+Shift+S` *(inferred)* | `WF-003` |
| `File > Save a Copy` | Dialog | — | Does not change document format |
| `File > Save for Web & Devices` | Dialog | `Ctrl/Cmd+Shift+Alt+S` *(inferred)* | `WF-005` |
| `File > Revert` | Command | `F12` | Reloads last saved version |
| `File > Close` | Command | `Ctrl/Cmd+W` *(inferred)* | Prompts if dirty |
| `File > Close All` | Command | — | *(inferred; not in Help PDF)* |
| `File > Exit` (`Quit`) | Command | `Ctrl/Cmd+Q` *(inferred)* | Closes app; prompts per dirty doc |
| `Edit/PS > Preferences > File Handling` | Preference panel | `Ctrl/Cmd+K` | Compatibility, save-in-background, recovery, recent count |
| Document window status bar | Status widget | Click / `Ctrl`-click | Document Sizes / Profile / Dimensions / etc. |
| `Window > Info` | Panel | `F8` *(inferred)* | Duplicate status options |
| Window title bar | Indicator | — | Dirty/unsaved marker *(inferred glyph)* |
| `Shift+Ctrl+W` / `Shift+Command+W` | Command | — | Close file and open Bridge |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Maximize PSD and PSB File Compatibility | enum | Always *(inferred)* | Always / Ask / Never | File Handling preference; Ask/Never cut file size |
| Save In Background (CS6) | bool | On *(inferred)* | on / off | Help recommends off for consistent performance on large files |
| Automatically Save Recovery Information (CS6) | enum | 10 minutes | interval list (10 min default) | File Handling; Help couples it to Save In Background |
| Recent File List Contains | int | *(unresolved)* | positive integer | Number of Open Recent entries |
| Image Previews | enum | Ask When Saving | Never Save / Always Save / Ask When Saving | File Handling |
| File Extension (Windows) | enum | *(unresolved)* | Use Upper Case / Use Lower Case | File Handling |
| Append File Extension (macOS) | enum | *(unresolved)* | Never / Always / Ask When Saving | File Handling |
| Save As to Original Folder | bool | On *(inferred)* | on / off | Default save location behavior |
| Revert | command | — | `F12` | One undoable History state |
| Duplicate Merged Layers Only | bool | Off | on / off | Image > Duplicate |
| Untitled document naming | string | `Untitled-N` *(inferred)* | auto-increment | Not stated in Help |
| Document status readout | enum | Document Sizes *(inferred)* | Sizes / Profile / Dimensions / Scratch / Efficiency / Timing / Tool / Measurement Scale | Status bar menu |
| 32-bit Exposure | slider | 0 *(inferred)* | HDR docs only | Preview-only |

## Algorithms & pipeline

### Lifecycle state machine

```text
            New/Open/Duplicate
  (none) ───────────────────────▶ Untitled ◀──┐ (Save As → path/format)
                                    │  │       │
                       edits ───────┘  └───────┘
                                    ▼
                                  Dirty ──Save──▶ Clean(path) ──edits──▶ Dirty
                                    │  ▲                                   │
                        Revert ─────┘  └──────────── Revert ──────────────┘
                                    │
                        Close/Exit ─┴─▶ (prompt if Dirty) ──▶ (none)
```

- **Clean ⇄ Dirty** is driven by the undo stack revision, not by a byte diff
  (`ARCH-009`).
- A **Save** writes a snapshot of the current document; editing continues in the
  live document. Background save must therefore either copy-on-write the
  serialized state or serialize from a locked snapshot so the write sees a
  consistent revision.
- **Revert** reloads the file into the existing document identity and seeds a
  History state; it does not create a new window.

### Background save (CS6)

1. `File > Save` (or Save As) constructs an immutable `SaveRequest { document_revision,
   path, format, options }`.
2. A worker serializes to a temporary file in the destination directory, then
   atomically renames over the target on success.
3. The UI stays live; on completion the document's `saved_revision` advances to
   the revision that was serialized. If the live revision has moved on, the
   document remains dirty.
4. Failures (disk full, permission denied, target deleted) surface a non-blocking
   error and leave `saved_revision` unchanged.

*(inferred — Help documents only the existence and preference of background save,
not the mechanism.)*

### Auto-recovery (CS6)

- A timer at the configured interval snapshots dirty documents to a recovery
  store (scratch/temp location, implementation-defined).
- Recovery records carry at least: original path (if any), format, document
  revision, and the serialized bytes. On next launch the recovery store is
  scanned; each record produces a recoverable document that the user may save or
  discard. Clean documents need no record.
- On graceful exit the recovery records for documents the user saved are cleared.

### Dirty-state and close prompts

- Each `DocumentView` shows the dirty marker from `Document.dirty`.
- Closing or quitting iterates dirty documents. `Save` may be asynchronous under
  background save, so the close flow must wait for the pending write or prompt
  again if it fails.

### Status computation

- **Document Sizes**: flatten the layer tree to estimate the PSD composite size,
  and separately estimate the full layered/channel byte size. Both are estimates
  in Help, so parity tolerance is loose (see acceptance criteria).
- **Scratch Sizes** and **Efficiency** map to the scratch-cache accounting in
  `01-architecture/performance-targets.md` and `scratch-disks-and-memory.md`.

## Rust module mapping

Proposals. Layered on the document model (`ARCH-008`):

- `pictura_shell::document::DocumentManager` — owns the set of open
  `DocumentHandle`s, the active handle, recent-file list, and close/quit
  orchestration (dirty prompts).
- `pictura_shell::document::DocumentHandle` — `{ id: DocId, state: DocumentState,
  doc: pictura_core::Document }`, where `DocumentState` holds
  `{ path: Option<PathBuf>, format: Option<FormatId>, saved_revision: Revision,
  dirty: bool, save_settings: SaveSettings, recovery: RecoveryState }`.
- `pictura_io::save::SaveTask` — background serializer: `SaveRequest`,
  `SaveProgress`, `SaveError`; writes via temp file + atomic rename.
- `pictura_io::save::atomic_write` — shared helper: temp file in destination dir,
  `fsync`, `rename`.
- `pictura_recovery::RecoveryStore` — append/replace interval snapshots, scan on
  startup, clear on clean exit. `RecoveryRecord { origin: Option<PathBuf>,
  format: FormatId, revision: Revision, payload: Vec<u8> }`.
- `pictura_shell::status::DocumentStatus` — computes Document Sizes / Profile /
  Dimensions / Scratch / Efficiency for the status bar and Info panel.
- `pictura_core::history::Revision` — reuse the undo stack revision from
  `ARCH-009` as the dirty/saved-revision token.

Types crossing the Qt boundary: `DocId`, `DocumentState` (read-only to the UI),
`SaveProgress` events, `RecoveryRecord` summaries.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `DocumentWindow` | `QMainWindow` (or `QMdiSubWindow`) | Canvas host; title-bar dirty marker; status bar |
| `DocumentTabBar` | `QTabBar` | Tabbed document switching; close buttons |
| `DocumentStatusBar` | `QStatusBar` | Magnification, file size, status menu, 32-bit exposure |
| `SaveProgressIndicator` | `QProgressBar` / status widget | Non-blocking background-save progress |
| `CloseConfirmDialog` | `QMessageBox` | Save / Don't Save / Cancel per dirty document |
| `RecoveryDialog` | `QDialog` | List recovered documents; save or discard |
| `RecentFilesMenu` | `QMenu` | `File > Open Recent`; syncs with `QSettings` |
| `DocumentStateModel` | `QObject` properties | Dirty/format/path exposed to QML or widgets |

Widgets over QML for the window/status chrome (dense, platform-integrated,
keyboard-centric) per `01-architecture/qt6-ui-design.md`. Recovery and close
dialogs are modal `QDialog`s. `DocumentState` is exposed read-only; all mutations
go through `DocumentManager` commands so undo and dirty tracking stay single-sourced.

## Data-model impact

- New per-document, non-serialized state: `path`, `format`, `saved_revision`,
  `dirty`, `save_settings` (last format + per-format option set), and
  `recovery_state`. None of these are written into the PSD/PSB payload; they are
  session state.
- **Save settings** are per document (a format choice and its options), not
  global, so that `File > Save` on a document saved as TIFF can repeat the TIFF
  options without re-prompting *(inferred)*.
- **Undo granularity**: Revert is exactly one undoable History state. Save does
  not add a History state. Auto-recovery snapshots do not touch the undo stack.
- **Recovery records** are parallel state, not part of `Document`; they may
  outlive a crash. The recovery serializer reuses the native serializer.
- **XMP/PSD**: opening a file may populate metadata and a color profile; that is
  `ARCH-008`/`ARCH-011`. Lifecycle adds nothing to the file format.

## Edge cases

- **Untitled documents**: no path/format; Save routes to Save As. Closing an
  untitled dirty document must prompt.
- **Read-only / locked files**: Save fails; background save must report it without
  losing the in-memory document. Save As to a writable location remains possible.
- **Disk full during save**: atomic temp-file + rename prevents a truncated
  original; report and keep the document dirty.
- **Target file deleted/moved between Save and write**: recreate if permitted,
  otherwise error.
- **Background save + concurrent edit**: the saved revision must be the snapshot
  revision, not the live one; document stays dirty if edits landed after the
  snapshot.
- **Multiple documents with unsaved changes on quit**: prompt in a defined order
  (tab order), and support Save All / Discard All if implemented (parity
  unconfirmed).
- **Auto-recovery vs manual save race**: a clean exit cancels pending recovery
  writes; a crash leaves the most recent interval snapshot.
- **Revert on an untitled document**: nothing to revert to; command disabled.
- **PSB / >30,000 px / >2 GB**: PSB supports any file size; TIFF caps at 4 GB;
  Photoshop Raw flattens. Background save on huge documents is exactly where the
  Help warning applies.
- **CMYK / Lab / 32-bit**: lifecycle is mode-agnostic; format constraints are
  enforced in `WF-003`/`ARCH-011`.
- **GPU unavailable**: no lifecycle impact.
- **Scratch disk full**: recovery snapshots may fail; degrade without data loss
  in the live document.
- **Crash during a background save**: the partial temp file must be ignored/
  cleaned; recovery record supplies the unsaved edits.

## Parity acceptance criteria

- Given a new untitled document, editing then `File > Save` opens Save As; after a
  successful Save As the title dirty marker clears and `File > Save` no longer
  prompts for a name.
- Given a saved document with edits, `File > Revert` restores the last saved
  pixels and adds exactly one undoable History state; `Edit > Undo` returns to
  the pre-Revert state.
- Given a document saved in the current format, `File > Save` does not re-prompt
  for format options; `File > Save As` does.
- Given a PSD with layers and Maximize Compatibility = **Always**, the saved file
  contains a merged composite; with **Never** it does not, and the file is smaller.
- Given CS6 auto-recovery enabled at interval T, a simulated crash after unsaved
  edits yields a recoverable document on next launch whose pixels match the last
  interval snapshot within tolerance.
- Given Save in Background enabled and a multi-hundred-megabyte document, the UI
  accepts edits while the save is in progress and reports completion without
  blocking.
- Given a dirty document, `File > Close` prompts with Save / Don't Save / Cancel;
  Cancel leaves the document open and dirty.
- Given `Image > Duplicate` with merged-layers-only off, the duplicate has the
  same layer count as the source; with it on, a single flattened layer.
- Given a document with layers and channels, the status-bar Document Sizes left
  number approximates the flattened PSD size and the right number approximates
  the layered size within a stated tolerance.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — the
  CS6 Help corpus, downloaded and text-extracted for this pass. Established:
  `File > Save`/`Save As` write-target semantics and the warning when a format
  cannot hold all document features; `Image > Duplicate` and
  **Duplicate Merged Layers Only**; **Save in Background (CS6)** and its File
  Handling preference with the large-file performance warning; **Auto-recover**
  (default ten minutes, configurable under File Handling); the Maximize PSD and
  PSB compatibility options **Always / Ask / Never** and the earlier-version
  feature-discard note; large-document limits (PSB any size, Photoshop Raw
  flattened, TIFF 4 GB); File Handling options (Image Previews, File Extension,
  Append File Extension, Save As to Original Folder, Recent File List Contains);
  the `Shift+Ctrl+W` / `Shift+Command+W` close-and-open-Bridge shortcut;
  `File > Revert` (`F12`) becoming a History state; the Info-panel and status-bar
  document status readouts (Document Sizes left/right, Profile, Dimensions,
  Scratch Sizes, Efficiency, Timing, Current Tool, Measurement Scale, 32-bit
  Exposure) and the `Ctrl`/`Command`-click tile-size readout.
- `https://html.duckduckgo.com/html/?q=Photoshop+CS6+New+Document+dialog+options+artboard+recent`
  — search-result page (results seen as snippets, not individually fetched);
  surfaced the Adobe community thread
  `https://community.adobe.com/t5/photoshop-ecosystem-discussions/how-to-put-artboards-in-photoshop-cs6/m-p/10833212`
  stating artboards are a CC (post-CS6) feature. Cross-checked against the CS6
  Help PDF, which mentions "Artboard" only as an Illustrator-only Save for Web
  control. Relevant to `WF-002` and the repository `GLOSSARY.md` contradiction
  noted below.

Internal cross-references (not external URLs): `docs/01-architecture/file-formats.md`
(`ARCH-011`) for per-format limits and the read/write matrix; `docs/GLOSSARY.md`
for term definitions.

## Open questions

- **Dirty-state indicator glyph.** The Help PDF never states how an unsaved
  document is marked in the title bar. Resolve by screenshot/behavior test of a
  shipped CS6 build (asterisk vs. platform dot vs. "Save changes?" only).
- **Close All / Quit prompt flow.** Whether CS6 offers a multi-document Save All
  and the exact prompt order is not documented. Resolve from a CS6 build or a CS6
  user guide snapshot.
- **Auto-recovery coupling.** Help's instruction ("Select Save In Background, and
  then select an interval from the Automatically Save Recovery Information menu")
  implies the recovery interval control depends on the background-save checkbox.
  Confirm whether recovery can run with background save off. Resolve from the
  actual File Handling dialog or a CS6 preferences reference.
- **Default values.** Defaults for Maximize Compatibility (assumed Always), Save
  In Background (assumed On), Image Previews (Ask When Saving is stated), recent
  list length, and extension case are not all stated in the Help PDF. Resolve by
  inspecting a default CS6 preferences file.
- **Save settings persistence scope.** Whether last-save-format options persist
  per document, per format, or in the app preferences is unverified. Resolve by
  editing and re-saving the same document in CS6.
- **Recovery storage location/format.** Help does not name the recovery path or
  file form. Resolve by inspecting a machine that has crashed CS6.
- **Revert granularity with background save.** Whether Revert is blocked while a
  background save is in flight is unverified.
- **Artboards.** The repository `GLOSSARY.md` calls artboards a CS6 feature, but
  the CS6 Help PDF does not document artboards for Photoshop and community
  sources place artboards in CC 2015. Resolve authoritatively before any artboard
  work; this directly affects `WF-002`'s "artboard-less" claim and
  `05-layers/artboards.md`.
- **Qt6 implementation details.** The proposed `QMainWindow`/`QMdiSubWindow` and
  status-bar APIs are design sketches; verify against Qt6 current docs before
  implementation.
