# Design: isolation-mode

## Context

Photoshop's isolation mode is not a separate canvas state: it is the Layers
panel **Selected** filter. CC-era sources describe two entry points — the
Layers panel filter popup's `Selected` type, and (with the Move tool active) the
canvas right-click menu's `Isolation Mode` — one removal command, `Release from
Isolation` in a layer's row context menu, and one exit, clearing the filter
(lightswitch off / another type). The repo already implements the filter
(`layers_filter_bar.cpp`, `layers_filter_proxy.cpp`) and the Move Auto-Select
(`helpers_composite.rs::auto_select_path`), so this change extends both rather
than adding a parallel mode.

The CS6 Help does not list `Selected` among the six filter types, so
`docs/05-layers/layer-filtering-and-search.md` excludes it as CC-only. The
contract is amended (with `TASK-ALLOWS-DOCS`) to bring it into scope.

## Approach

- **The filter dimension.** `kDimensions` in `layers_filter_bar.cpp` gains
  `{"selected", "Selected"}` with an empty criteria page (the criterion is the
  selection, not a widget). `LayerFilter` gains `bool selected` and a
  `QSet<QString> selectedPaths`; `LayersFilterProxyModel::hasActiveCriteria` and
  `rowMatches` treat `selected` as a criterion that matches when the row's
  `PathRole` is in `selectedPaths`. The panel rebuilds `selectedPaths` from
  `selectedPaths()` and re-applies the filter whenever the selection changes, so
  the view stays live.
- **Move Auto-Select scoping.** The panel pushes the isolated set to the view
  (`isolation_set(paths, active)`) whenever the Selected dimension is active and
  enabled, and clears it otherwise. `PictureViewRust.isolation: Option<Vec<String>>`
  holds it; `auto_select_path` takes the set and only returns a hit whose path is
  a member or a descendant of a member. `auto_select_layer` passes
  `view.rust().isolation`. `reset_edit_state` clears it, so a document switch
  leaves isolation.
- **Release from Isolation.** A bespoke row-menu item added only while isolation
  is active, above the common block. It deselects that row in the panel (which
  removes it from `selectedPaths`); when the selection empties, the panel turns
  the filter off, ending isolation.
- **Enter / exit.** The panel exposes `enterIsolation()` (select the `Selected`
  dimension and enable the toggle) and `releaseIsolation()` (turn the filter
  off). The frame's Move-tool canvas context menu calls `enterIsolation()` on the
  current selection; `PicturaMainWindow::keyPressEvent` calls
  `releaseIsolation()` on Escape, before the screen-mode branch.

## Alternatives

- **A separate `isolation` mode flag.** Rejected: Photoshop's isolation *is* the
  Selected filter, and a second state would have to stay in sync with the filter
  and the selection.
- **Confining paint/filter edits and dimming the canvas (the issue's literal
  wording).** Rejected as not Photoshop behavior; the issue's "edits apply only
  within it" maps to the scoped Move Auto-Select, and there is no canvas dim in
  Photoshop. See `proposal.md`.
- **A full Move-tool canvas layer-picker menu.** Photoshop's canvas right-click
  menu lists the layers under the cursor; only the `Isolation Mode` entry is
  implemented here, leaving the picker list a follow-up.

## Verification

- A Rust `#[test]` for `auto_select_path` scoping: an unscoped hit returns the
  topmost layer; a scoped hit returns only a member/descendant; a hit outside the
  set returns none.
- Qt Test: the `Selected` dimension shows only the selected rows and their
  ancestors, the Release row removes a layer and ends isolation on the last,
  Escape clears the filter, and the filter toggle-off restores every row.
- `tst_command_tree` unchanged (no menu-bar command is added); `bash
  scripts/verify-fast.sh`; `openspec validate --all --strict`.
