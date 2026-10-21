# Layer Comps

- **Spec ID:** `LAY-022`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — layer comps themselves date from Photoshop CS; CS6 restores **Layer Comps to PDF** as a `File > Scripts` option (JDI list). Smart-object/layer-comp integration and bulk update across comps are **post-CS6** (CC 2014). See `## Open questions`.
- **Depends on:** `ARCH-008` document-model, `ARCH-009` undo-history, `LAY-020` smart-objects, `LAY-021` smart-filters, `02-ui-ux/panels/layers-panel.md`, `09-automation/script-events-and-jsx.md`, `10-workflow-io/export-formats.md`

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is taken from the fetched CS6 Help PDF unless
> marked *(inferred)*.

## CS6 behavior

A **layer comp** is a saved snapshot of the state of the Layers panel. Designers use comps to create, manage, and view multiple versions of a layout inside one Photoshop file. A comp records exactly three kinds of layer option:

1. **Layer visibility** — whether a layer is showing or hidden.
2. **Layer position in the document.**
3. **Layer appearance** — whether a layer style is applied to the layer, and the layer's **blending mode**.

> Help explicitly notes that, unlike layer effects, **Smart Filter settings cannot be changed across layer comps**: once a Smart Filter is applied to a layer it appears in every comp for that image.

### Create

1. `Window > Layer Comps` to show the **Layer Comps** panel.
2. Click **Create New Layer Comp**; the new comp reflects the current Layers-panel state.
3. In the **New Layer Comp** dialog, name the comp, add descriptive comments, and choose the options to record: **Visibility**, **Position**, **Appearance**.
4. `OK`. The chosen options are stored as defaults for the next comp.
   - To duplicate a comp, select it and drag it to the New Comp button.

### Apply and view

- Click the **Apply Layer Comp** icon next to a selected comp to apply it.
- Use the **Previous** / **Next** buttons at the bottom of the panel to cycle through comps (select specific comps first to cycle only those).
- Click **Apply Layer Comp** next to **Last Document State** at the top of the panel to restore the document to its state before a comp was applied.

### Change and update

1. Select the comp.
2. Make changes to layer visibility, position, or style (you may need to enable the matching recording option).
3. To change what the comp records, `Layer Comp Options` from the panel menu, and enable **Position** and/or **Appearance**.
4. Click **Update Layer Comp**.

### Warnings ("Layer Comp Cannot Be Fully Restored")

Certain actions make a comp impossible to restore fully — deleting a layer, merging a layer, or converting a layer to a background. A **caution icon** then appears next to the comp name. The user can:

- Ignore the warning (may lose one or more layers; other saved parameters may survive).
- Update the comp (loses the previously captured parameters but brings it current).
- Click the caution icon to read the explanation and choose **Clear** to remove the alert without changing the remaining layers.
- Right-click / Control-click the caution icon for `Clear Layer Comp Warning` or `Clear All Layer Comp Warnings`.

### Delete

Select the comp and click the panel Delete icon, or `Delete Layer Comp` from the panel menu, or drag the comp to the Delete icon.

### Export / scripts

- `File > Scripts > Layer Comps to Files` — exports layer comps to individual files; choose the file type and destination.
- `File > Scripts > Layer Comps to PDF` — CS6 restores this as a Scripts option (JDI list).
- `File > Scripts > Layer Comps to WPG` — exports to a Web Photo Gallery; requires the optional WPG plug-in from the Goodies folder on the installation disc.
- Panel keyboard support (Help): `Alt`/`Option`-click New Layer Comp = create without the dialog; double-click comp = open Layer Comp Options; double-click comp name = rename inline; `Shift`-click = select a contiguous range; `Ctrl`/`Cmd`-click = toggle discontiguous selection.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Layer Comps` | Panel | — | Shows/hides the Layer Comps panel |
| Layer Comps panel — Create New Layer Comp | Button | `Alt`-click | Creates a comp; `Alt`-click skips the dialog |
| Layer Comps panel — Apply Layer Comp | Icon/button | — | Applies the selected comp |
| Layer Comps panel — Last Document State | Row | — | Restore pre-comp document state |
| Layer Comps panel — Previous / Next | Buttons | — | Cycle comps |
| Layer Comps panel — Update Layer Comp | Button | — | Re-capture current state |
| Layer Comps panel — Delete | Icon | — | Deletes a comp |
| Layer Comps panel — caution icon | Indicator | `Right-click` | Warning actions |
| Layer Comps panel menu | Menu | — | `New Layer Comp`, `Layer Comp Options`, `Delete Layer Comp`, `Clear … Warnings` |
| New Layer Comp / Layer Comp Options dialog | Dialog | `Double-click` comp | Name, Comment, Visibility, Position, Appearance |
| `File > Scripts > Layer Comps to Files` | Menu (script) | — | Export each comp to a file |
| `File > Scripts > Layer Comps to PDF` | Menu (script) | — | CS6-restored option |
| `File > Scripts > Layer Comps to WPG` | Menu (script) | — | Requires optional Web Photo Gallery plug-in |
| Layers panel | Panel | `F7` | Source of the state the comp captures |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Comp name | string | "Layer Comp N" | free text | Shown in the panel list |
| Comment | string | empty | free text | Descriptive comment |
| Visibility | bool | on | on / off | Record layer show/hide |
| Position | bool | off | on / off | Record layer position; stored as next-comp default |
| Appearance | bool | off | on / off | Record layer style applied + blending mode |
| Comp selection | list state | — | single / contiguous / discontiguous | `Shift`-click / `Ctrl`-click |
| Last Document State | implicit | — | — | Always present at the top of the panel |
| Warning state | enum | clean | clean / cannot-be-fully-restored | Set by delete/merge/background conversion |

## Algorithms & pipeline

### Capture model

A comp stores a **diff** of per-layer captured state rather than a full copy of the document *(inferred; the observable behavior is a snapshot of the three option classes)*:

```text
LayerComp {
  id, name, comment,
  record: { visibility: bool, position: bool, appearance: bool },
  entries: Map<NodeId, CapturedState>,
}
CapturedState {
  visible: Option<bool>,          // if record.visibility
  position: Option<Vec2>,         // if record.position
  style_applied: Option<bool>,    // if record.appearance
  blend: Option<BlendMode>,       // if record.appearance
}
```

**Apply** walks the entries and writes the captured values back into the layer tree (visibility, translation, style on/off, blend mode). **Update** re-captures the current state for the comp's recording options.

### Identity and warnings

Because a comp references layers by identity, deleting/merging a layer or converting it to a background makes the captured entry unresolvable. Photoshop detects this at apply/preview time and marks the comp with the caution icon. *(inferred)* the layer's stable **layer ID** — the document ID seed in image resource 1044 and the `lyid` layer ID — is the right identity key, not the layer index or name. The `Layer Group(s) Enabled ID` resource (1072) is the analogous per-layer flag array and may carry comp/group state.

### Serialization

Layer Comps are stored **document-wide in image resource 1065 (`0x0429`)**, documented as "(Photoshop CS) Layer Comps. 4 bytes (descriptor version = 16), Descriptor." *(sourced from the PSD file-format specification)*. The descriptor contents (the comp list and per-layer captured values) are not enumerated in the fetched spec text; see `## Open questions`. Smart Object/stack-filter state is not part of a comp (`LAY-021`).

### Scripts

`Layer Comps to Files` / `to PDF` / `to WPG` enumerate the comps, apply each in turn, render, and write. In an ExtendScript/JSX world this is exposed through the DOM; Kooka Pictura's scripting host is owned by `09-automation/script-events-and-jsx.md`. *(inferred)* the same operations should be exposed as host commands so batch/actions can record them.

## Rust module mapping

- `pictura_core::layercomp::LayerComp` — `{ id, name, comment, record: CompRecord, entries: HashMap<NodeId, CapturedState> }`.
- `pictura_core::layercomp::CompRecord` — `{ visibility: bool, position: bool, appearance: bool }`.
- `pictura_core::layercomp::CapturedState` — optional visible/position/style_applied/blend.
- `pictura_core::layercomp::CompStore` — document-level list; `capture(tree, record)`, `apply(tree, comp)`, `update(tree, comp)`, `validate(tree, comp) -> Warning`.
- `pictura_io::psd::layer_comps` — read/write image resource 1065 descriptor; preserve unknown descriptor keys verbatim.
- `pictura_script::commands::layer_comps` — export-to-files / export-to-PDF / export-to-WPG command surface for actions/scripts.

Crossing types: `CompId`, `NodeId(u64)`, `BlendMode`, `Vec2`, and a `CompSummary { id, name, comment, warning }` list model handle.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `LayerCompsModel` | `QAbstractListModel` | Comp list: name, comment, selected, warning state, "Last Document State" pseudo-row |
| `LayerCompsPanel` | `QWidget` | Panel with Apply/Previous/Next/Update/Delete/Create buttons and the model view |
| `LayerCompOptionsDialog` | `QDialog` | Name, Comment, Visibility, Position, Appearance |
| `LayerCompWarningIndicator` | `QStyledItemDelegate` | Caution icon + context actions (Clear, Clear All) |
| `ExportLayerCompsDialog` | `QDialog` | Destination + file type for the Files/PDF/WPG scripts |
| `LayersModel` bridge | `QAbstractItemModel` | Applies/reports visibility/position/style/blend so the Layers panel reflects an applied comp |

Widgets over QML per `ARCH-003`: the panel is a dense, keyboard-centric dock; the same model can back a QML view if the shell goes that way.

## Data-model impact

- A document-level `Vec<LayerComp>` (plus an implicit "Last Document State") lives alongside metadata; it is not a node in the layer tree.
- Comp capture references nodes by stable **layer ID**; the document ID seed (resource 1044) and `lyid` layer IDs must be preserved across round-trips (`ARCH-008`).
- Undo: applying a comp is an undoable "set multiple layer states" command (one history state, not one per layer); create/update/delete/rename/options are separate commands. Applying a comp must snapshot the pre-apply state so `Last Document State` (and undo) can restore it.
- Serialization: image resource 1065 descriptor (version 16). Unknown descriptor keys are preserved as opaque bytes.
- Interaction: Smart Filter settings are **not** comp-varied; Smart Object internal layer comps and cross-comp bulk update are CC 2014 features, not CS6 (`LAY-020`, `LAY-024`).
- The comp list is per-document and not written to XMP.

## Edge cases

- **Deleted layer** — entry unresolved; caution icon; ignore/update/clear paths must all be supported.
- **Merged layer** — the merged target's captured state applies; the absorbed layer's entry is stale; caution icon.
- **Layer converted to Background** — same warning path.
- **New layer added after capture** — not represented; it keeps its current state unless the comp is updated.
- **Groups** — group visibility/position/appearance are captured like any layer; the `Layer Group(s) Enabled ID` resource (1072) may be the serialization vehicle *(inferred)*.
- **Smart Filters** — cannot vary by comp; changing a filter affects all comps.
- **Smart Objects** — visibility/position/appearance captured; internal comps and Convert/Embed Linked are post-CS6.
- **Nested groups / pass-through** — appearance capture records the group's own blend mode, not the composite interaction.
- **Large documents** — capture should store diffs, not whole documents; apply must be O(captured entries), not O(layers).
- **No comps** — the panel still shows only "Last Document State"; export scripts must handle the empty case.
- **Undo/redo across apply** — repeated apply/undo must be lossless and not accumulate drift.
- **Missing script plug-ins** — WPG export requires an optional plug-in; fail with a clear message, do not crash.
- **PSD round-trip** — resource 1065 must survive even when the engine interprets none of the comps.
- **Localization** — default comp names and the "Last Document State" label are localized (see `11-cross-cutting/localization.md`).

## Parity acceptance criteria

- Given a document with layers `[A visible, B hidden, C visible]`, creating a comp with Visibility on, then hiding A and showing B and applying the comp, restores `[A visible, B hidden, C visible]` exactly.
- Given a comp created with Appearance on, changing a layer's blend mode/style then applying the comp restores the captured blend and style state.
- Given a comp created with Position on, moving a layer then applying the comp restores its captured position within tolerance.
- Given a comp created without Position recorded, moving a layer then applying the comp does **not** reset its position.
- Given `Last Document State`, applying it after a comp restores the pre-apply document exactly (within tolerance) and is undoable as one step.
- Given a Smart Filter applied to a layer, applying different comps does not vary the Smart Filter settings.
- Given `Update Layer Comp` after a change, the comp now reflects the current state.
- Given a deleted layer referenced by a comp, the caution icon appears and `Clear Layer Comp Warning` removes the icon without altering the remaining layers.
- Given `File > Scripts > Layer Comps to Files` on a document with three comps, three files are written, each rendering the corresponding comp.
- Given `File > Scripts > Layer Comps to PDF`, the output has one page/region per comp (CS6-restored option).
- Given a CS6-authored PSD containing layer comps, opening and re-saving preserves image resource 1065 and the comps are applicable.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — the CS6 Help corpus. Established: the definition of a layer comp and the three recorded option classes (visibility, position, appearance); the Smart Filter note (settings cannot vary across comps); create/apply/view/update/clear-warning/delete/duplicate workflows and exact button names; the caution-icon triggers (delete, merge, convert to background) and the Clear/Clear All warning actions; export via `File > Scripts > Layer Comps to Files` and `Layer Comps to PDF`; the optional WPG export plug-in note; the Layer Comps panel shortcut table; the CS6 JDI note "Layer Comps to PDF restored as a Scripts option".
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/` — Adobe PSD/PSB File Formats Specification. Established: image resource **1065 (`0x0429`), "(Photoshop CS) Layer Comps. 4 bytes (descriptor version = 16), Descriptor"**; related resources 1044 (layer ID seed), 1069 (Layer Selection ID(s)), 1072 (Layer Group(s) Enabled ID), and the `lyid` layer ID key.
- `https://bjango.com/articles/photoshopcc2014smartobjects/` — established that cross-comp bulk update and Layer Comps inside Smart Objects are **CC 2014** features, not CS6; used to bound the CS6 scope.

Not used in this pass:

- `helpx.adobe.com` (HTTP 403) — modern Help pages were inaccessible; the archived CS6 Help PDF was used instead.

## Open questions

- **Resource 1065 descriptor schema.** The spec states the resource is a version-16 descriptor but does not enumerate its comp-list keys or the per-layer captured fields. Resolve by parsing CS6-authored PSDs with layer comps (`psd-tools`/`PhotoshopAPI` support is partial).
- **Layer identity in comps.** Whether CS6 keys comps by layer ID (`lyid`), the 1044 seed, or a comp-internal index is unconfirmed. Resolve with a controlled test: delete a layer, add another, and inspect resource 1065.
- **Exact "appearance" capture set.** Help names layer style on/off and blending mode. Whether opacity, fill, clipping, mask enable/disable, or layer effects parameters are also captured is unverified. Resolve with a state-matrix test.
- **Position representation.** Whether position is stored as absolute canvas coordinates, layer offset, or a transform is not documented. Resolve with a move-and-capture test.
- **`Last Document State` persistence.** Whether the pre-comp state is serialized in the PSD or only held in memory is unconfirmed; this decides whether it survives save/reopen.
- **Smart Object internal comps.** CC 2014 exposes comps inside a Smart Object to the host; confirm CS6 does not, and ensure the data model leaves room without implementing it.
- **Script DOM surface.** The exact ExtendScript names for layer comp operations and whether actions can record comp apply/create are not sourced. Resolve from the CS6 Scripting Guide.
