# View Menu View-State Wiring

## Why

The View menu still exposes 27 disabled leaves from the CS6 contract
([#503](https://github.com/KookaStudio/KookaPictura/issues/503), filed from the
[#486](https://github.com/KookaStudio/KookaPictura/issues/486) parity audit).
Users cannot toggle non-printing extras individually or with `Ctrl+H`, cannot
show or hide grids, pixel grids, layer edges, target paths, slices, notes or
counts with another tool active, cannot lock or clear slices, and cannot jump to
`200%` or `Print Size` from the menu. `Proof Setup` and `Proof Colors` already
shipped with the PDF writer change (#626), so this change covers the remaining
view-state family.

Snap and Snap To belong to [#305](https://github.com/KookaStudio/KookaPictura/issues/305);
32-bit Preview Options belongs to [#570](https://github.com/KookaStudio/KookaPictura/issues/570).
Gamut Warning, Color Blindness proofing and Pixel Aspect Ratio remain separate
changes.

## What Changes

- Wire `View > Extras` (`Ctrl+H`), the `View > Show` submenu, `Show > All` and
  `Show > None`, and add `Show > Show Extra Options` as the window that selects
  which extras the `Ctrl+H` master hides. Per-item state, the master, and the
  membership set persist and apply to every canvas.
- Add the missing overlays: Grid (with the Guides, Grid, & Slices grid
  preferences group), Pixel Grid above 500% zoom, and Layer Edges for the active
  layer.
- Make Target Path, Slices, Notes and Count persist with tools other than their
  own; gate them, Guides, Smart Guides, Selection Edges and color samplers
  through `Extras` / `Show` / `All` / `None`.
- Follow CS6 for `Show > Selection Edges`: it affects the current selection, and
  a new selection shows the edges again.
- Add `View > Lock Slices` and `View > Clear Slices`; `Clear Slices` removes
  every user slice in one history state and leaves the derived auto slice.
- Add `View > 200%` and `View > Print Size` with the already-implemented zoom
  tool behavior (`200%`; `96 / ppi` for Print Size).
- Remove the phantom `View > Align` and `View > Align To` entries, which
  `docs/02-ui-ux/menus.md` records as not being Photoshop menu items.
- Update the docs contract (Clear Slices, Show Extra Options semantics), the
  parity register row UI-7, and `ROADMAP.md`.

Explicitly out of scope: Snap/Snap To (#305), 32-bit Preview Options (#570),
Gamut Warning, Color Blindness proofing, Pixel Aspect Ratio, the CC-only
additions (Flip Horizontal/Vertical, Pattern Preview, Fit Layers/Artboard, New
Guide Layout), and 3D Axis (no 3D engine; it stays visible and disabled).

## Capabilities

### New Capabilities

- `ui/view-menu`: the application-wide View overlay state, the Extras master and
  membership window, the Show submenu entries introduced here (Selection Edges
  scope, Target Path, Layer Edges, Grid, Pixel Grid), color-sampler gating, and
  the `200%` / `Print Size` View commands.

### Modified Capabilities

- `tools/slice-tool`: show resolved slices with any tool when `Show > Slices` is
  on; add `Lock Slices` and `Clear Slices`.
- `tools/count-tool`: gate count marks with `Show > Count` and `Extras` while
  other tools are active.
- `tools/note-tool`: gate note glyphs with `Show > Notes` and `Extras` while
  other tools are active.
- `tools/smart-guides`: `Extras` also hides smart guides and stops snapping,
  without changing the Show toggle.
- `ui/rulers-and-guides`: `Extras` hides guides without changing the Show
  toggle; placing a guide while hidden turns Extras on; the Grid preferences
  group becomes live (Smart Guides and Slices stay CS6-disabled).

## Impact

- Command catalogue and generated identifiers: `pictura-script` entries and
  `command_ids.h`.
- C++ shell: menu handlers, session state, canvas overlays, Slice tool,
  Preferences, and a new Show Extra Options window.
- Engine: `pictura-render` gains `clear_slices`; `pictura-app` exposes the clear
  bridge through `crop_group`.
- Session: new persisted view-state fields (per-item flags, Extras membership,
  grid settings); no document pixels, history, or saved-file changes.
- Tests: a new `tst_view_menu` Qt suite, Rust clear-slice cases, and updates to
  the existing rulers, smart-guides, command-tree, slice and annotation suites
  where their scenarios change.
- No new dependency.
