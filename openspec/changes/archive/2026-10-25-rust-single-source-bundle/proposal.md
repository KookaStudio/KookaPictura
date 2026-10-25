# Proposal

## Why

The Qt shell keeps its own copies of data and math that the Rust engine already
owns. These are the blend-mode list, the colour-model conversions, the adjustment
descriptors, and the tool catalogue. Each copy can drift from the engine
without anything failing, and the colour copies already disagree. The Info panel
computes Lab against D65, the picker uses D50, and both compute CMYK with a
fixed formula, which is not the working-space profile transform the colour
contract calls for. The `type_api` adapter, added by #276, mostly forwards calls
one to one, which is the pass-through shape the view-seam contract rejects.
This change makes Rust the single source for these tables and this math. The
Qt widgets stay in C++.

## What Changes

- **#271:** The bridge exposes the engine's layer blend-mode list (PSD key and
  CS6 name, in `BlendMode::LAYER_MODES` order). `kBlends` and `blend_modes.h`'s
  table are deleted, and every layer blend combo, filter and label builds from
  the engine list.
- **#269:** `pictura-color` gains public scalar conversions: sRGB↔Lab (D50,
  Bradford-adapted ICC connection space) and sRGB↔CMYK through the CMYK working
  profile (relative colorimetric, black-point compensation). The colour picker
  and the Info panel (Lab, CMYK, Total Ink) call them through the bridge. The
  C++ copies are deleted: `rgbToLab`/`labToRgb`, the Info panel's D65
  `srgbToLab`, and `QColor::toCmyk`. The Channels panel's `toCmyk` is a copy of
  the engine's plate split, which channel-to-selection already uses in Rust. It
  is deleted in favour of that engine function, and the thumbnails stay
  unchanged. **Behaviour change:** Info panel Lab now matches the picker (D50),
  and CMYK readouts follow the profile rather than the naive formula.
- **#272:** Rust is the canonical adjustment catalogue. One table gives, for
  each kind, its title, its Image > Adjustments slot (dialog or direct
  command, or an Image-root command for the Auto items), its New
  Fill/Adjustment Layer entry, and its Adjustments panel tile and menu label.
  Parameter ranges and defaults come only from the Rust adjustment layout. The
  C++ tables `kDialogs`, the direct-command table, `kLayerAdjustments` and the
  Adjustments panel table are deleted. So are the hard-coded ranges in
  `makeAdjustmentDialog` and in the hand-laid dialogs: exposure, hue/saturation,
  levels, curves, colour balance, black & white, photo filter, channel mixer
  and gradient map.
- **#274:** The tool catalogue (id name, label, group, default shortcut, cursor,
  hotspot, hint, modifier hints, implemented flag) moves into Rust. The C++
  `ToolId` enum, handlers, options bars and toolbox rendering stay in C++. A
  test checks the `ToolId` order against the Rust catalogue using its own
  enumerator table. `allToolIds` derives from the catalogue, and the unused
  `implementedToolIds` is deleted.
- **#403:** `type_api` is deepened, not folded. Its interface stops mirroring
  the bridge:
  - one `TypeSpec` value replaces the five-argument text/font/setting tuple;
  - an edit session object owns begin, commit and cancel;
  - a warp session owns preview, commit and cancel;
  - style libraries are read in one call;
  - the per-field getters collapse into `readLayer`.

  The generated-header exception stays confined to `type_api.h`. The two other
  headers that include `type_tools.cxxqt.h` directly, and `layers_api.cpp`,
  move behind the adapter.
  The decision is recorded in the view-seam record.

## Capabilities

### New Capabilities

- `color/color-model-conversions`: the shared sRGB↔Lab and sRGB↔CMYK
  conversions, and the rule that every colour readout uses them.
- `imaging/adjustment-catalogue`: one engine-owned catalogue of adjustment kinds
  and parameter specs, which every adjustment menu, panel and dialog consumes.
- `tools/tool-catalogue`: one engine-owned tool catalogue, which the toolbox,
  shortcuts, hints, search and control server consume.

### Modified Capabilities

- `compositing/blend-modes`: adds the requirement that the engine's layer-mode
  list is the only blend-mode table the UI uses.
- `ui/info-histogram-panel`: the CMYK readout comes from the shared
  working-space conversion instead of a local formula.
- `ui/view-seam-adapters`: adds the type-unit requirement. The adapter is a
  session façade, not a forwarder, and it is the only includer of the type
  generated header.

## Impact

- Rust: `pictura-core` (blend list accessor), `pictura-color` (new `model`
  conversions), `pictura-render` (adjustment catalogue beside
  `ADJUSTMENT_DIALOG_KINDS`), `pictura-script` (tool catalogue beside the
  command catalogue), and `pictura-app` bridges: new `blend_modes`,
  `color_models`, `adjustment_catalogue` and `tool_catalogue` bridge modules.
  `pictura-app` gains a path dependency on the workspace crate `pictura-color`.
  No third-party dependency is added.
- C++: `blend_modes.h`, the layer combos (new layer dialog, Layer Style,
  Layers panel, filter bar, Properties panel), `color_picker_dialog.*`,
  `info_panel.cpp`, `channels_panel.cpp`, `frame_menus_adjust.cpp`,
  `layer_adjustments.h`, `adjustments_panel.cpp`, `adjustment_dialog.cpp` and
  the hand-laid adjustment dialogs, `tools.h`, `tool_catalog.cpp`,
  `tool_registry.h`, `api/type_api.*` and its about 35 callers, `type_frame.h`,
  `character_panel.h`, and `CMakeLists.txt` for new files.
- Tests: new Rust unit tests for each table and the conversions, a new
  `tst_blend_mode_list` suite, and new or updated Qt Test cases. The
  `tst_info_panel` expectations change only where the colour behaviour changes.
  About 20 type suites migrate mechanically to the deepened `type_api`, with
  their assertions kept. The headless self-test exit codes stay the same (98,
  116, 117 and 536).
- Docs (`TASK-ALLOWS-DOCS` commits):
  - `docs/dev/refactor-view-seam.md` records the #403 decision.
  - The `kToolTable` references in `docs/GLOSSARY.md`,
    `docs/02-ui-ux/search-bar.md` and `docs/dev/refactor-tool-architecture.md`
    follow the move.
  - `docs/dev/STATE.md` is updated. `docs/dev/doc-code-parity-gaps.md` row CLR-7 (#604)
  narrows to the gamut triangle and the 16/32-bit fields, because CMYK now
  goes through the working profile.
- Out of scope: #275, #277, #278, #280, #783, #359, the non-layer paint-mode
  combos (options bar, Fill, Stroke), the picker/Info gamut alert, and document
  profile–aware readouts.
