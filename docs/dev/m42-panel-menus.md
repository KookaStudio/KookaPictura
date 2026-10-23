# M42 — per-panel CS6 panel menus (research)

- **Status:** proposed reference for `openspec/changes/m42-panel-refinements`.
  This file is the frozen contract for the per-widget tab-header menus (M42
  item 11). It is not code.
- **Consumers:** `crates/pictura-app/cpp/panels/panel_group.{h,cpp}` (the
  tab-header action button), and the per-panel menu table
  (`panels/panel_menus.{h,cpp}`, new, or folded into the header button).

## Provenance and confidence

The entry lists below were researched from Adobe's Photoshop CS6 Help
(`https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf`) plus
CC-era UI screenshots for ordering. Two caveats are load-bearing:

1. **Ordering is best-effort.** Adobe's Help documents a panel's commands by
   *function* (creating, editing, deleting, display options, panel options)
   rather than printing the live menu order, and the CC-era screenshots are a
   later release. The grouping and the separator positions below are a faithful
   reconstruction, not a pixel-verified CS6 dump. A CS6 screenshot can reorder
   rows later without any code-structure change.
2. **Not every documented panel is a CS6 panel.** Gradients, Patterns, and
   Libraries are explicitly marked below as non-goals for this reason.

Status marks:

- `[impl]` — the action exists today and the menu entry is wired to it.
- `[disabled]` — the feature is not implemented; the entry ships disabled with
  the tooltip `<label> — not implemented yet` (the M38/M40 convention).
- `[tab-menu]` — deliberately **not** in the per-widget menu; it lives on the
  group tab context menu (`Close`, `Close Panel Group`).
- `[toggle]` / `[radio]` — the CS6 control type; a disabled entry keeps the
  checkable/radio affordance but cannot be toggled until implemented.

`Panel Options…` is a single entry inside the per-widget Layers menu; it must
keep calling the existing M39/M41 `LayersPanel::openPanelOptions` behaviour and
must not regress it.

## Layers

Source: CS6 Help "Layers panel overview" / "Layer menu"; order cross-checked
against CC screenshots. (Layers is the only panel with a substantial implemented
subset from M39/M41.)

| Entry | Status | Wired to |
|---|---|---|
| New Layer… | `[impl]` | `LayersPanel::addLayerAt` |
| Copy CSS | `[disabled]` | — |
| Duplicate Layer/Group… | `[impl]` | `duplicateSelection` |
| Delete Layer/Group | `[impl]` | `deleteSelection` |
| Delete Hidden Layers | `[disabled]` | — |
| New Group… | `[impl]` | `addGroupAt` |
| New Group from Layers… | `[disabled]` | — |
| Lock Layers… | `[disabled]` | — |
| Convert to Smart Object | `[disabled]` | M46 |
| Rasterize Layer | `[disabled]` | — |
| Group Layers | `[impl]` | `groupSelection` |
| Ungroup Layers | `[impl]` | `ungroupSelection` |
| Hide Layers | `[impl]` | visibility toggle |
| Arrange ▸ | partial | see sub-entries |
| ↳ Bring to Front / Bring Forward / Send Backward / Send to Back / Reverse | `[disabled]` | — |
| ↳ Move Layer Up / Move Layer Down | `[impl]` | `moveCurrent(±1)` |
| Link Layers | `[disabled]` | — |
| Select Linked Layers | `[disabled]` | — |
| Merge Down / Merge Visible / Merge Clipping Mask | `[disabled]` | — |
| Flatten Image | `[disabled]` | — |
| Blending Options… | `[disabled]` | M45 |
| Animation Options ▸ | `[disabled]` | — |
| Panel Options… | `[impl]` | `openPanelOptions` |
| Close / Close Tab Group | `[tab-menu]` | tab context menu |

## Channels

Source: CS6 Help "Channels panel overview" / channels panel menu. The Channels
panel is a placeholder in M42; every entry is disabled.

| Entry | Status |
|---|---|
| New Channel… | `[disabled]` |
| Duplicate Channel… | `[disabled]` |
| Delete Channel | `[disabled]` |
| New Spot Channel… | `[disabled]` |
| Merge Spot Channel(s) | `[disabled]` |
| Split Channels | `[disabled]` |
| Merge Channels… | `[disabled]` |
| Channel Options… | `[disabled]` |
| Panel Options… | `[disabled]` |
| Close / Close Tab Group | `[tab-menu]` |

## Paths

Source: CS6 Help "Paths panel overview" / paths panel menu. Placeholder panel;
every entry is disabled.

| Entry | Status |
|---|---|
| New Path… | `[disabled]` |
| Duplicate Path… | `[disabled]` |
| Delete Path | `[disabled]` |
| Save Path… | `[disabled]` |
| Make Work Path… | `[disabled]` |
| Make Selection… | `[disabled]` |
| Fill Path… / Fill Subpath… | `[disabled]` |
| Stroke Path… / Stroke Subpath… | `[disabled]` |
| Clipping Path… | `[disabled]` |
| Panel Options… | `[disabled]` |
| Close / Close Tab Group | `[tab-menu]` |

## Color

Source: CS6 Help "Color panel overview" / "Using the Color panel". The M20 Color
panel exists but exposes fixed sliders; the menu modes are disabled.

| Entry | Status |
|---|---|
| Sliders ▸ | `[disabled]` |
| ↳ Grayscale / RGB / HSB / CMYK / Lab / Web Color Sliders | `[disabled]` |
| RGB Spectrum | `[disabled]` |
| CMYK Spectrum | `[disabled]` |
| Grayscale Ramp | `[disabled]` |
| Current Colors | `[disabled]` |
| Make Ramp Web Safe | `[disabled]`, `[toggle]` |

## Swatches

Source: CS6 Help "Swatches panel overview" / swatches panel menu.

| Entry | Status |
|---|---|
| New Swatch… | `[disabled]` |
| Display ▸ | `[disabled]` |
| ↳ Small Thumbnail / Large Thumbnail / Small List / Large List | `[disabled]` |
| Preset Manager… | `[disabled]` |
| Load Swatches… / Save Swatches… | `[disabled]` |
| Save Swatches For Exchange… | `[disabled]` |
| Replace Swatches… | `[disabled]` |
| Reset Swatches | `[disabled]` |
| (libraries list) | `[disabled]` |

## Styles

Source: CS6 Help "Styles panel overview" / styles panel menu. The Styles panel
is a placeholder in M42.

| Entry | Status |
|---|---|
| New Style… | `[disabled]` |
| Display ▸ | `[disabled]` |
| ↳ Text Only / Small Thumbnail / Large Thumbnail / Small List / Large List | `[disabled]` |
| Preset Manager… | `[disabled]` |
| Load Styles… / Save Styles… | `[disabled]` |
| Replace Styles… | `[disabled]` |
| Reset Styles | `[disabled]` |
| (libraries list) | `[disabled]` |

## Navigator

Source: CS6 Help "Navigator panel overview". The M20 Navigator panel exists; its
options dialog is not implemented.

| Entry | Status |
|---|---|
| Panel Options… | `[disabled]` |

## Histogram

Source: CS6 Help "Histogram panel overview" / histogram panel menu. The M20
Histogram panel can refresh, but none of these menu affordances is exposed.

| Entry | Status |
|---|---|
| Uncached Refresh | `[disabled]` |
| Compact View / Expanded View / All Channels View | `[disabled]`, `[radio]` |
| Show Channels In Color | `[disabled]`, `[toggle]` |
| Show Statistics | `[disabled]`, `[toggle]` |

## Info

Source: CS6 Help "Info panel overview" / info panel menu.

| Entry | Status |
|---|---|
| Panel Options… | `[disabled]` |
| Color Samplers | `[disabled]`, `[toggle]` |

## History

Source: CS6 Help "History panel overview" / history panel menu. The M20 History
panel has undo/redo; the documented step commands map to the existing
`EditStepForward`/`EditStepBackward` commands.

| Entry | Status |
|---|---|
| Step Forward | `[impl]` (existing command) |
| Step Backward | `[impl]` (existing command) |
| New Snapshot… | `[disabled]` |
| Delete | `[disabled]` |
| Clear History | `[disabled]` |
| New Document | `[disabled]` |
| History Options… | `[disabled]` |

## Actions

Source: CS6 Help "Actions panel overview" / actions panel menu. The Actions
panel and engine shipped in `#301`; the panel group asks the `ActionsPanel` to
fill this table on `aboutToShow`, so enablement and checks are dynamic.

| Entry | Status |
|---|---|
| Button Mode | `[impl]`, `[toggle]` |
| New Action… / New Set… | `[impl]` |
| Duplicate / Delete / Play | `[impl]` |
| Start Recording (Stop Recording while recording) / Record Again… | `[impl]` |
| Insert Menu Item… / Insert Stop… | `[impl]` |
| Insert Conditional… | `[impl]` — **Kooka extension**, not CS6 |
| Insert Path | `[impl]` |
| Action Options… / Set Options… | `[impl]`, contextual on selection |
| Playback Options… | `[impl]` |
| Allow Tool Recording | `[impl]`, `[toggle]`, off by default |
| Clear All Actions / Reset Actions | `[impl]` |
| Load Actions… / Replace Actions… / Save Actions… | `[impl]` |
| Installed `.atn` sets | `[impl]` — appended rows from `Presets/Actions` |
| Close / Close Tab Group | `[impl]` — appended by the panel group, exactly once each |

Live `.atn` and `Actions Palette.psp` I/O is owned by `09-automation/actions.md`;
the menu does not duplicate its format rules. Load, Replace, Reset, and Save are
disabled while recording or playing; Play is disabled while recording; Start
Recording is disabled while playing; editing rows require the selection named in
`docs/02-ui-ux/panels/actions-panel.md`.

## Adjustments

Source: CS6 Help "Adjustments panel overview" / adjustments panel menu. The M4
adjustment kinds that the Layers panel already exposes are implemented; the rest
of the M4 catalogue is reachable only through the Layers panel's fill/adjustment
button today.

| Entry | Status |
|---|---|
| Invert / Posterize / Threshold / Brightness-Contrast / Hue-Saturation | `[impl]` (existing adjustment kinds) |
| Remaining CS6 adjustment list | `[disabled]` |
| Add Mask by Default | `[disabled]`, `[toggle]` |
| Clip to Layer | `[disabled]`, `[toggle]` |

## Properties

Source: CS6 Help "Properties panel overview" / properties panel menu (CC-era
screenshots for ordering). The Properties content folds into `Adjustments` in
M41; its panel is a placeholder, so every entry is disabled.

| Entry | Status |
|---|---|
| Save Preset… / Save Black & White Preset | `[disabled]` |
| Auto Options… / Curves Display Options… | `[disabled]` |
| Show Clipping For Black/White Points | `[disabled]`, `[toggle]` |
| Auto-Select Parameter | `[disabled]`, `[toggle]` |
| Auto-Select Targeted Adjustment Tool | `[disabled]`, `[toggle]` |
| Apply Mask / Delete Mask / Disable Mask (mask mode) | `[disabled]` |

## Gradients, Patterns, Libraries — non-goals

These are **not CS6 panels** and are not given a real per-widget menu:

- **Gradients** and **Patterns** were picker pop-ups in CS6 (the gradient and
  pattern pickers), not dockable panels. The in-app Gradients and Patterns
  placeholders are M24/M41 empty states; their per-widget menus are disabled
  stubs with this note as the reason, not a feature backlog.
- **Libraries** is a CC-only panel (Adobe Creative Cloud Libraries) that did not
  exist in CS6. The Libraries placeholder keeps a disabled stub menu only.

Any future work to give these panels content is a separate change, not an M42
deliverable.
