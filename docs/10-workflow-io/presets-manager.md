# Presets Manager

- **Spec ID:** `WF-020`
- **Status:** `Draft`
- **Parity tier:** `Core` — the Preset Manager, its eight preset types, and `.abr`/`.aco`/… libraries ship in CS6 Standard and Extended.
- **New in CS6:** `Changed` — the menu path moves from `Edit > Preset Manager` (CS5) to `Edit > Presets > Preset Manager` (CS6); CS6 adds `Edit > Presets > Migrate Presets` (import CS3+ presets) and `Edit > Presets > Export/Import Presets` (share a setup across workgroup machines). The dialog itself, its display modes, and the `Load`/`Replace`/`Reset`/`Save Set` operations are CS5-equivalent.
- **Depends on:** `BRU-006` brush-presets, `CLR-003` swatches-and-libraries, `07-color-painting/gradient-presets.md`, `07-color-painting/pattern-presets.md`, `05-layers/layer-styles.md`, `09-automation/actions.md` (`AUTO-001`), `02-ui-ux/panels/tool-presets-panel.md`, `02-ui-ux/preferences.md` (`UI-010`), `01-architecture/file-formats.md`, `11-cross-cutting/preference-storage.md`.

> All crate, module, widget, and type names below are **design proposals**.
> The repository contains a working implementation; this file stays the
> behavioural contract for the feature, and `docs/dev/doc-code-parity-gaps.md`
> records what shipped and what did not. Facts confirmed by the fetched CS6
> Help reference are stated plainly; community/`.psp`-documented facts are
> marked *(secondary)*; inferred design choices are marked *(inferred)*.

## CS6 behavior

The **Preset Manager** manages the shipped libraries of preset brushes,
swatches, gradients, styles, patterns, contours, custom shapes, and preset
tools. It edits the *current set* of preset items per
type and can save, load, append, replace, reset, rename, delete, and reorder
them. After a library is loaded in the Preset Manager, its items become
available across the options bar, panels, dialog boxes, and so on.

### Opening and preset types

- CS6 path: `Edit > Presets > Preset Manager`. CS5 path was
  `Edit > Preset Manager` (the CS6 Help still prints both).
- The **Preset Type** menu selects one of eight managed types:
  `Brushes`, `Swatches`, `Gradients`, `Styles`, `Patterns`, `Contours`,
  `Custom Shapes`, `Tools` (tool presets).
- Additional preset kinds exist in panels but are not all Preset Manager types
  (e.g. Actions are managed in the Actions panel, `AUTO-001`).

### Display modes

Chosen from the top of the panel menu: `Text Only`, `Small Thumbnail`,
`Large Thumbnail`, `Small List`, `Large List`, and — for brush presets only —
`Stroke Thumbnail` (sample stroke + thumbnail). Items are reordered by dragging
up or down in the list.

### Rename / delete

- **Rename:** select item(s) → `Rename`; or double-click (thumbnail modes →
  dialog, list/text modes → inline edit).
- **Delete:** select → `Delete`; or `Alt`/`Option`-click the item(s).

### Libraries: load, append, replace, reset, save

| Command | Behavior |
|---|---|
| Library name at the bottom of the panel menu | Choosing it prompts OK (= replace the current list) or Append. |
| `Load` | Adds a library file to the current list. |
| `Replace [Preset Type]` | Replaces the current list with a chosen library. |
| `Save Set` | Writes all selected items (all, or a Shift-selected subset) to a library file in the type's format. |
| `Reset` | Restores the default items; user chooses replace or append. |
| Drop-in discovery | A library placed in the default `Presets/<type>` folder appears at the bottom of the panel menu after Photoshop is restarted. |

Each type has its own file extension and default folder. Newly created presets
live in the **Preferences file** until saved as a library; a preferences reset
(or deleting the prefs file) loses them. This matches `BRU-006` and `CLR-003`.

### Migration and sharing (new in CS6)

- `Edit > Presets > Migrate Presets` imports brushes, swatches, gradients,
  patterns, and other presets from Photoshop CS3 or later; the command prompts
  before importing from CS4/CS5.
- `Edit > Presets > Export/Import Presets` packs a custom preset setup for a
  workgroup. (The Help names the commands; their exact archive format is not
  documented in the CS6 Help.)

### Preset file formats

The CS6 Help names the operations but not the extensions. The following is the
community-documented map (see Sources); treat the byte layouts as behavioral
targets, not contracts.

| Preset type | Extension | Panel / command |
|---|---|---|
| Brushes | `.abr` | Brush Presets panel (`BRU-006`) |
| Color swatches | `.aco` | Swatches panel (`CLR-003`) |
| Swatch exchange | `.ase` | `Save Swatches For Exchange` (`CLR-003`); *community-sourced* |
| Layer styles | `.asl` | Styles panel |
| Gradients | `.grd` | Gradients panel |
| Patterns | `.pat` | Patterns panel |
| Custom shapes | `.csh` | Shapes panel |
| Contours | `.shc` | Layer Style contour pop-up |
| Curves | `.acv` | Curves adjustment Load/Save |
| Color books (spot) | `.acb` | Color Libraries picker |
| Actions | `.atn` | Actions panel (`AUTO-001`) |
| Tool presets | `.tpl` | Tool Presets panel / Preset Manager Tools |
| Keyboard shortcuts | `.kys` | `Edit > Keyboard Shortcuts` (`WF-021`) |
| Menu customization | `.mnu` | `Edit > Menus` (`WF-021`) |
| (Panels, unlabeled) | `.psp` files | Preferences store, e.g. `Brushes.psp`, `Swatches.psp`, `ToolPresets.psp` |

The internal serialization of most of these formats is proprietary and only
partly publicly documented (see `BRU-006` for the `.abr` analysis). The
`.aco`/`.ase` swatch formats are the only ones the community treats as
documented exchange formats.

### Where presets live on disk (CS6, sourced)

The CS6 Help's "Default preset locations" section:

| Platform | Default preset location |
|---|---|
| macOS | `<User>/Library/Application Support/Adobe/Adobe Photoshop CS6/Presets` |
| Windows XP | `[Drive]:\Documents and Settings\<user>\Application Data\Adobe\Adobe Photoshop CS6\Presets` |
| Windows Vista/7/8 | `[Drive]:\Users\<user>\AppData\Roaming\Adobe\Adobe Photoshop CS6\Presets` |
| Shipped presets | the `Presets` folder inside the Photoshop program folder |

The current Adobe "preference file functions, names, locations" article (which
the community confirms still matches CS6 by substituting the version) refines
this:

- Shipped: `Applications/Adobe Photoshop [version]/Presets/[type]` (macOS) /
  `C:\Program Files\Adobe\Adobe Photoshop [version]\Settings` (Windows).
- User presets: `~/Library/Application Support/Adobe/Adobe Photoshop
  [version]/Presets/[feature]` (macOS) / `Users\<user>\AppData\Roaming\Adobe\
  Adobe Photoshop [version]\Presets\[feature]` (Windows).
- Per-type panel lists are `*.psp` files under the version's **Settings**
  folder: `Brushes.psp`, `Swatches.psp`, `Gradients.psp`, `Patterns.psp`,
  `Styles.psp`, `CustomShapes.psp`, `Contours.psp`, `ToolPresets.psp`,
  `Actions palette.psp`.

*(secondary)* The shipped list is version-templated in the modern article; the
CS6 Help itself says only that it is inside the Presets folder of the Photoshop
application folder.

### Proposed Linux mapping

| Role | Proposed path |
|---|---|
| Shipped (system) | `/usr/share/kooka-pictura/presets/<type>/` |
| Shipped (Flatpak) | `/app/share/kooka-pictura/presets/<type>/` |
| User libraries | `$XDG_DATA_HOME/kooka-pictura/presets/<type>/` (`~/.local/share/...`) |
| Session/edited panel lists | `$XDG_CONFIG_HOME/kooka-pictura/presets/<type>.preset-list` |
| Discovery precedence | user dir first, then system, then Flatpak |
| Migration source | CS6 `Presets/` folder, opt-in import only (`WF-023`, independent-creation) |

See `01-architecture/build-and-packaging.md` for the Flatpak/AppImage path
policy.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Edit > Presets > Preset Manager` | Dialog | — | CS6 path (CS5: `Edit > Preset Manager`). |
| Preset Type | Drop-down | — | Eight managed types. |
| Panel menu — display modes | Menu | — | Text/Thumb/Large/List/Stroke Thumbnail (brush only). |
| Panel menu — `Load` / `Replace [Type]` / `Reset` | Menu items | — | Append vs replace semantics. |
| `Save Set` | Button | — | Saves all or Shift-selected items. |
| `Rename` / `Delete` | Buttons | — | `Alt`/`Option`-click deletes a row. |
| Preset list | List/grid | — | Drag to reorder; multi-select. |
| Panel fly-outs (Brush/Swatch/…) | Menus | — | Per-type `Load`/`Replace`/`Save`/`Reset`; library names at bottom. |
| `Edit > Presets > Migrate Presets` | Menu | — | CS6; import CS3+ presets. |
| `Edit > Presets > Export/Import Presets` | Menu | — | CS6; workgroup setup sharing. |
| Filesystem `Presets/<type>` | Files | — | Drop-in library appears in menu after restart. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Preset Type | enum | last used | Brushes/Swatches/Gradients/Styles/Patterns/Contours/Custom Shapes/Tools | Preset Manager page selector. |
| Display mode | enum | Small Thumbnail *(inferred)* | Text Only / Small Thumbnail / Large Thumbnail / Small List / Large List / Stroke Thumbnail | Stroke Thumbnail is brush-only. |
| Selection | set | single | single / Shift-range / Ctrl-toggle | Drives Save Set subset. |
| Preset name | string | generated | any Unicode | Renamable. |
| Library path | path | type default folder | any | Only the default folder is auto-discovered after restart. |
| Reset mode | enum | prompt | replace / append | `Reset`. |
| Load mode | enum | append | append / replace | Library entry ⇒ replace or append. |
| Migration source | enum | none | CS3/CS4/CS5 | `Migrate Presets`. |

## Algorithms & pipeline

The Preset Manager is a **registry + codec + persistence** layer; there are no
image algorithms.

### Preset lifecycle

```text
load_library(type, path):
    bytes = read(path)
    lib   = codec[type].decode(bytes)        # .abr/.aco/.ase/.asl/...
    registry[type].append(lib.items)         # Load
    # or: registry[type].items = lib.items  # Replace
save_set(type, items, path):
    bytes = codec[type].encode(items)        # native and/or Adobe format
    atomic_write(path, bytes)                # QSaveFile semantics
reset(type, mode):
    defaults = shipped_library(type)
    mode == replace ? registry[type] = defaults
                    : registry[type].extend(defaults)
apply(type, item):
    emit PresetSelected { type, id }         # consumed by tool/panel
```

### Codec strategy (proposal)

- **Read:** parse each Adobe format on a best-effort basis and report partial
  parses (matching `BRU-006`: Adobe's ActionDescriptor encoding is not reliably
  writable).
- **Round-trip:** use an **open, versioned native format** per preset type for
  lossless internal storage, with Adobe formats as import/export adapters. This
  is a deliberate deviation from byte-level parity and is recorded as such.
- **Shared descriptor codec:** `.abr`, `.asl`, and many others embed the same
  OSType/ActionDescriptor tree; implement it **once** in
  `pictura-presets::codec::ostype` and reuse it (also used by `AUTO-001` for
  `.atn`).
- **Discovery:** scan shipped + user `presets/<type>` directories at startup and
  on demand; watch for changes (per filesystem) rather than polling.

### Migration

`Migrate Presets` is an opt-in independent importer that copies recognizable
files from a user-supplied CS3+ presets tree into the user preset dir,
converting what the codecs understand and flagging what they do not. The CS6
`Export/Import Presets` bundle is a separate proposal: a versioned archive (e.g.
`kooka-pictura-presets-<version>.tar.zst`) containing the user preset tree plus a
manifest. *(inferred; CS6's bundle format is undocumented.)*

## Rust module mapping

Proposals; shares a crate with `BRU-006` and `CLR-003`.

- `pictura-presets::registry` — `PresetRegistry` mapping `PresetType` to an
  ordered `PresetLibrary<T>`; `load`, `append`, `replace`, `reset`, `save_set`,
  `rename`, `delete`, `reorder`.
- `pictura-presets::model` — `PresetType` enum; trait `Preset { fn id(&self);
  fn name(&self); fn thumbnail(&self) -> Option<GrayBitmap>; }`.
- `pictura-presets::codec` — dispatch by `PresetType`/magic; submodules
  `abr` (`BRU-006`), `aco`, `ase`, `asl`, `grd`, `pat`, `csh`, `shc`, `acv`,
  `tpl`, plus `ostype` (shared ActionDescriptor tree) and `native`.
- `pictura-presets::discovery` — scans `XDG_DATA_DIRS` preset roots; returns
  `DiscoveredLibrary { type, path, source: Shipped|Flatpak|User }`.
- `pictura-presets::migrate` — independent-creation CS3+ importer (`Migrate Presets`).
- `pictura-presets::bundle` — `Export/Import Presets` archive (manifest +
  payload); versioned.
- `pictura-presets::prefs` — session-persistent preset lists (`*.psp`
  equivalent) in the preference store.
- `pictura-presets::events` — `PresetSelected` / `PresetLibraryChanged`
  notifications consumed by panels and tools.

Crossing types: `PresetType`, `PresetId(u64)`, `PresetLibrary<T>`,
`LibrarySource`, `CodecReport { parsed: usize, failed_at: Option<u64> }`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `PresetManagerDialog` | `QDialog` | Preset Type selector + list + action buttons |
| `PresetTypeComboBox` | `QComboBox` | The eight managed types |
| `PresetManagerModel` | `QAbstractListModel` | Items, thumbnails, check/selection, drag-reorder |
| `PresetItemDelegate` | `QStyledItemDelegate` | Thumbnail/text/list modes; Stroke Thumbnail for brushes |
| `PresetManagerMenu` | `QMenu` | Display modes; Load/Replace/Reset; library entries |
| `PresetLibraryFileDialog` | `QFileDialog` | Type-filtered load/save; default `presets/<type>` dir |
| `MigratePresetsDialog` | `QDialog` | Choose CS3+ source tree; per-type result report |
| `PresetImportReport` | `QDialog` | Partial-parse diagnostics for community formats |

Widgets, not QML: dense, keyboard-driven desktop dialog consistent with
`ARCH-003`. Thumbnails are rendered in Rust and surfaced as `QImage`; all file
I/O runs off the GUI thread; long scans report progress and are cancellable.

## Data-model impact

- **Not document data.** Preset libraries and their `*.psp`-equivalent session
  lists live in the preference/preset stores; a PSD does not embed them
  *(inferred)*.
- **No undo:** preset edits are application-state mutations, not document
  History states (matching CS6, where program-wide changes are not reflected in
  the History panel).
- **Preference records:** `{ preset_type, display_mode }`,
  `{ preset_type, session_items: Vec<PresetId> }`, and per-type library order.
- **Preset-file serialization:** Adobe formats (read + best-effort write) and
  the open native format; unknown/unsupported fields are preserved opaquely on
  re-save.
- **Migration/bundle:** written outside the document model; never referenced by
  PSD/XMP.

## Edge cases

- **Corrupt or unknown library version:** keep what parsed, report the failing
  section/offset, never crash (`BRU-006` precedent).
- **Partial ActionDescriptor parse:** community parsers fail partway; surface a
  "partially imported" flag rather than dropping the file.
- **Duplicate names / UUIDs:** disambiguate on load (`name`, `name 2`); never
  silently overwrite.
- **Preferences reset:** unsaved session presets are lost; warn before reset.
- **Default-folder convention:** a library outside the type folder loads but is
  not auto-listed after restart; match or document a refresh.
- **Very large libraries:** virtualize the list; loading must not block the GUI.
- **Concurrent edits:** do not write a library while a preset is being applied;
  an active stroke must keep its captured configuration.
- **Unicode/locale names:** round-trip UTF-16/UCS-2 names correctly.
- **Type mismatch:** a `.abr` dragged onto the Swatches page is rejected with a
  clear message, not misparsed.
- **Read-only system dir:** fall back to the user preset dir; never fail at
  startup because a bundled library is unreadable.
- **Cross-version files:** record format version + unknown keys so a newer file
  re-saves without loss.

## Parity acceptance criteria

1. Given `Edit > Presets > Preset Manager`, the Preset Type menu lists exactly
   Brushes, Swatches, Gradients, Styles, Patterns, Contours, Custom Shapes, and
   Tools.
2. Given a CS6-era `.abr` (or `.aco`/`.asl`/`.grd`/`.pat`/`.csh`/`.shc`/`.acv`/
   `.tpl`) library, `Load` appends its items with names/thumbnails and
   `Replace` swaps the list.
3. Given a subset selection, `Save Set` writes a library containing exactly the
   selected items, and re-loading reproduces them (native round-trip exact;
   Adobe round-trip best-effort with documented gaps).
4. Given `Reset`, the shipped default library is restored as replace or append
   per the user's choice.
5. Given a library dropped into `presets/<type>`, its name appears in the
   panel/list after the equivalent of an application restart.
6. Given `Rename`/`Delete`, the change is reflected in the Preset Manager and in
   the owning panel or options-bar picker.
7. Given a malformed or newer library, import reports what parsed, flags the
   preset as partially imported, and does not crash.
8. Given `Migrate Presets` against a CS3/CS4/CS5 presets tree, supported types
   are copied and unsupported files are reported.
9. Given a preferences reset, newly created unsaved presets are gone and the
   user was warned beforehand.
10. Given a reordered list, the order survives a restart.

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official Photoshop CS6 Help (downloaded with `curl`, text-extracted with
  `pdftotext`). Establishes: `Edit > Preset Manager` (CS5) vs `Edit > Presets >
  Preset Manager` (CS6); the eight preset types; display modes and Stroke
  Thumbnail; drag-reorder; rename/delete; Load (add) vs Replace vs library-at-
  bottom-of-menu (OK/Append); Save Set subset; Reset (replace/append); the
  Preferences-file caveat; the default-folder auto-listing after restart; the
  statement that each library type has its own file extension and default folder;
  the default preset locations (Mac/XP/Vista) and shipped `Presets` folder;
  `Migrate Presets` and `Export/Import Presets`.
- `https://web.archive.org/web/20240419165453/https://helpx.adobe.com/photoshop/kb/preference-file-names-locations-photoshop.html`
  — Adobe "Preference file functions, names, locations". Establishes the
  version-templated shipped/user preset paths and the per-type `*.psp` panel
  files (`Brushes.psp`, `Swatches.psp`, `Gradients.psp`, `Patterns.psp`,
  `Styles.psp`, `CustomShapes.psp`, `Contours.psp`, `ToolPresets.psp`,
  `Actions palette.psp`) under the Settings folder. Direct `helpx.adobe.com`
  fetch returns HTTP 403; this is the Wayback capture.
- `https://www.gottheknack.com/a-user-guide/ps/ps-presets-list/ps-presets-list-01.html`
  — "A List of Photoshop Presets & Their File Extensions" (CC 2019, macOS):
  `.atn`, `.abr`, `.aco`, `.acv`, `.csh`, `.grd`, `.asl`, `.pat`, `.shc`,
  `.tpl`, `.mnu`, `.acb`, and others. Secondary/community.
- `https://photoshop-viz.blogspot.com/2013/03/photoshop-file-extensions.html` —
  extension-to-UI-path table (`ABR`, `ACO`, `ACV`, `ASL`, `ATN`, `CSH`, `GRD`,
  `PAT`, `SHC`, `TPL`, `KYS` keyboard shortcuts, `MNU` custom menus, `ACB`,
  `ACT`). Secondary/community.
- `https://doc.qt.io/qt-6/qsettings.html` — QSettings location rules and
  atomic/INI behavior used for the Linux preference-store proposal.
- `https://doc.qt.io/qt-6/qtemporaryfile.html` — atomic temp-file + rename
  semantics used for the library save proposal.
- Cross-references: `docs/07-color-painting/brush-presets.md` (`BRU-006`, `.abr`
  analysis), `docs/07-color-painting/swatches-and-libraries.md` (`CLR-003`,
  `.aco`/`.ase`), `docs/09-automation/actions.md` (`AUTO-001`, `.atn` shared
  descriptor codec), `docs/01-architecture/file-formats.md`.

## Open questions

- **CS6 preset file extensions.** The CS6 Help names operations, not extensions;
  the extension map above is community-sourced from later versions. *Resolves
  with:* a CS6 `Presets/` folder listing and a save/export capture.
- **Exact CS6 default preset folder.** The CS6 Help prints the CS5-era text with
  a CS5 example path; the version-templated modern article is corroboration, not
  proof. *Resolves with:* a CS6 install's actual folder tree.
- **`Export/Import Presets` bundle format.** Unspecified by the Help. *Resolves
  with:* capturing a CS6 export or Adobe's documentation for the command.
- **`Migrate Presets` mapping.** Which CS3/CS4/CS5 preset types and versions are
  actually converted, and which are skipped, is not documented. *Resolves with:*
  a CS6 migration experiment.
- **Native-format decision.** Whether Kooka Pictura should write Adobe formats at
  all (round-trip fidelity vs. independent-creation effort) is a product decision. See
  `00-overview/licensing-and-provenance.md` and `BRU-006`.
- **De-duplication on load/append.** Whether CS6 de-duplicates identical presets
  is unverified. *Resolves with:* a CS6 add-twice/load-twice test.
- **Display-mode default per type.** Assumed Small Thumbnail. *Resolves with:* a
  CS6 first-run capture.
