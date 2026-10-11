# Proposal: rulers-and-guides

## Why

Issues #294 and #299: Kooka Pictura has no rulers and no guides, and no
Units & Rulers preferences to choose the ruler unit. In CS6, `View > Rulers`
(`Ctrl+R`) shows a ruler along the top and left of every document window;
dragging out of a ruler drops a guide, a non-printing line drawn in cyan over
the canvas, and the guides' look is set in
`Edit > Preferences > Guides, Grid & Slices`
(`docs/02-ui-ux/menus.md` View, `docs/02-ui-ux/preferences.md`,
`docs/02-ui-ux/application-frame.md` "Place/Edit guides").

## What Changes

- **Rulers.** `View > Rulers` (`Ctrl+R`) toggles a horizontal ruler above and a
  vertical ruler left of every document canvas. They measure from the
  document's top-left corner in inches by default (CS6's default), label
  distances from 0 without a sign, follow pan and zoom, and mark the cursor
  position. A right-click picks the unit (pixels, inches, cm, mm, points,
  picas, percent). Visibility and unit are global and persisted; rulers are
  off by default, as in CS6.
- **Guides.** Dragging from the top ruler places a horizontal guide, from the
  left ruler a vertical one, at the whole document pixel under the release;
  releasing off the canvas places nothing. Guides are document data: each
  edit records one history state ("New Guide", "Move Guide", "Delete Guide",
  "Clear Guides") and they are saved in the PSD. With the Move tool (or any
  tool while `Ctrl` is held) a guide can be dragged to a new position, and
  dragging it off the canvas deletes it.
- **View menu.** `Show > Guides` (`Ctrl+;`), `Lock Guides` (`Alt+Ctrl+;`),
  `Clear Guides`, and `New Guide…` (orientation and position in pixels) become
  real. Show and Lock are global, persisted toggles.
- **Preferences.** `Edit > Preferences > Guides, Grid, & Slices` becomes a real
  page. Its Guides group sets the guide colour (CS6's presets plus Custom…,
  default Cyan) and style (Lines / Dashed Lines), persisted and applied to
  every canvas. The Smart Guides, Grid, and Slices groups are shown with their
  CS6 defaults but disabled until those features exist.
- **Units & Rulers (#299).** `Edit > Preferences > Units & Rulers` becomes a
  real page: its Rulers menu sets the ruler unit (in sync with the ruler
  context menu) and Point/Pica Size picks 72 or 72.27 points per inch; both
  persist. Double-clicking a ruler opens it. Type, Column Size, and New
  Document Preset Resolutions show CS6's defaults, disabled.
- **PSD.** Guides are written to and read from the grid-and-guides image
  resource (1032).

## Capabilities

### New Capabilities

- `ui/rulers-and-guides`: rulers, guide placement/editing, the View menu
  commands, and the Guides preferences.
- `codec/psd-guide-resources`: the grid-and-guides image resource.

## Impact

- `pictura-core`: `Guide`, `GuideOrientation`, `guide_near`; `Document.guides`.
- `pictura-codec`: new `guide_resources.rs`, hooked into read and write.
- `pictura-app` (Rust): new `cxxqt_object/guides.rs` bridge.
- `pictura-app` (C++): `canvas_ruler.*`, `image_view_guides.cpp`,
  `tools_guides.cpp`, `frame_guides.cpp`, `new_guide_dialog.*`; the canvas host
  gains the two rulers; a Guides preferences page; session keys
  `rulersVisible`, `rulerUnit`, `traditionalPoints`, `guidesVisible`, `guidesLocked`, `guideColor`,
  `guideDashed`.
- No new dependencies.

## Provenance

Behaviour from the docs cited above and the screenshots on issue #294 (the
Cyan and Magenta swatches were sampled from the CS6 preferences screenshot).
The 1032 layout follows the Adobe PSD file-format specification and is
verified against `psd-tools`; its 1/32-pixel location unit is the de-facto
convention, not stated by the specification. Ceilings (`ponytail:`): rulers
no columns ruler unit and no ruler-origin drag; the Units & Rulers Type,
Column Size, and preset resolutions are not yet wired; no
snapping (`View > Snap`, Shift-drag to ticks) and no Alt-drag to flip a
guide's orientation; guide colour presets other than Cyan and Magenta are
approximated.
