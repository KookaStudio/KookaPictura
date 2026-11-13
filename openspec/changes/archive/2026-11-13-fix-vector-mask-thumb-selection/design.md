# Design

## Context

See `proposal.md` for motivation and the delta spec for the required behavior.

The panel keeps a panel-local `activeThumbByPath_` (`QHash<QString, int>`) and
projects it into `LayersModel::Row::activeThumb`, exposed as `ActiveThumbRole`.
The delegate paints one bracket rect from that value. The bridge owns exactly
one mask edit target (`mask_edit_target`, raster-only) and clears it whenever the
active layer changes; there is no vector edit target because vector path tools
are not wired yet. `refresh` derives the active thumb from the bridge target, so
target and selection currently cannot disagree.

`refresh` sets `applyingTarget_` for its whole body and then calls
`projectTarget`, which applies an external layer-target change and updates the
`rememberedTarget_`/`rememberedAnchor_`/`rememberedGeneration_` fields directly.
External layer switches (canvas, menus, history restore) arrive through
`PictureView::changed`, so `rememberTarget` — which only runs from
`writeTarget`, `makeRowAnchor`, `writeCurrentAnchor`, and `selectPaths` — is not
a hook for every selection change.

Three files touched by the change are at a size boundary: `layers_panel.cpp` is
allowlisted at 1262 lines, `layers_panel_internal.h` is at 1199 of a 1200 cap,
and `tst_layers_panel.cpp` is allowlisted at 1408. `layers_panel_events.cpp`,
`layers_panel_target.cpp`, and `tst_canvas_mask_domain.cpp` have headroom.

## Goals / Non-Goals

**Goals:**

- Represent the content thumbnail, the raster mask, and the vector mask as one
  selection with exactly one companion and an optional vector mask.
- Keep the raster-mask edit target and the selected raster-mask thumbnail in
  sync, including when the vector mask is also selected.
- Clear the previous layer's selection when another layer becomes active,
  including when the switch comes from outside the panel.
- Keep `Shift`, `Ctrl`, and `Alt`+`Shift` clicks from changing the companion or
  vector selection.
- Add Qt coverage for the issue sequence, companion switches, inactive-row
  clicks, modifier clicks, the vector-only row, and render-level bracket checks.

**Non-Goals:**

- A bridge-level vector edit target or vector path editing.
- Changing the row-body default (it stays content, per the existing
  plain-row-click requirement).
- Changing the right-click mask menus' selection side effects.
- Multi-selection bracket rendering (only a singular active row draws brackets).

## Decisions

### One bitmask role instead of two booleans

`ActiveThumbRole` keeps returning an int, now interpreted as named bits:
`ActiveThumbContent = 1`, `ActiveThumbMask = 2`, `ActiveThumbVector = 4`. Valid
states are `1`, `2`, `5`, and `6`; `0` is only a transient no-selection value,
and the click paths always write `Content` (or the companion bits), so a
rebuilt row is always one of the four valid states. A second role or two
booleans would duplicate the existing model patch, role plumbing, and delegate
lookup. The legacy values (`0` content, `1` raster, `2` vector) cannot express
content+vector without a fourth flag and make the refresh branchy, so the
semantic values change and the test assertions are updated mechanically.

### The bridge target is authoritative for the companion

`refresh` normalizes each row with
`(activeThumb & ActiveThumbVector) | (row == maskTarget ? ActiveThumbMask : ActiveThumbContent)`.
The vector bit is the only selection state that survives a model rebuild; the
companion is read from the one existing source of truth. This keeps mask add,
filter commits, history restore, and layer-target changes from leaving a mask
bracket without a target (or the reverse), and needs no new bridge state.

### The vector toggle preserves the companion; inactive rows start from content

Clicking the vector thumbnail toggles its bit without choosing a companion.
The issue's "both masks selected" state follows from the raster mask already
being the companion after the masks are added. A thumb click on a row that is
not the layer-selection anchor starts from the content companion, ignoring that
row's stale bits; this yields content-only, raster-mask-only, or content+vector
for the three inactive-row thumb clicks and prevents a stale vector state from
resurrecting on a different layer.

### Shift, Ctrl, and Alt+Shift clicks never change the selection

A plain click writes the companion and vector bits, and an `Alt`-click on the
raster mask additionally selects the raster companion and its edit target
because it is the click that enters solo. The raster branch writes the bits only
after its early `Shift` return (the `Ctrl` and `Alt`+`Shift` branches are already
handled earlier), and the vector branch handles `Shift` before the plain toggle
so it only toggles the enabled bit. The `Ctrl` and `Alt`+`Shift` clicks keep
their existing mask-coverage and rubylith effects and leave the selection alone.
Activating an inactive row through any of these clicks may still default that
row to the content companion because the bridge clears the mask target when the
anchor changes.

### The raster edit target follows the companion

The raster mask is the paintable mask, so it is the edit target exactly while it
is the companion; the content companion clears the target. This reverses the
earlier decision that a vector-mask click drops the raster target (from
`layers-panel-manual-test-fixes`), because that made the two-mask selection
impossible. Consequence: clicking the vector mask on a both-mask row now keeps
the white/black mask color pair.

### Prune the per-row hash at the top of `refresh`

`projectTarget` handles programmatic and external target changes and bypasses
`rememberTarget`, and `refresh` builds its rows before it calls `projectTarget`.
Pruning in `rememberTarget` therefore misses the canvas/menu/history path and
would be one frame late. Prune `activeThumbByPath_` to
`layer_selection_anchor(*view_)` at the top of `refresh`, before the snapshot
and row loop, so every selection change is normalized in the same rebuild. The
old normalization block shrinks to one line, so the allowlisted
`layers_panel.cpp` does not grow. Also clear the hash in `refresh`'s existing
`newDocument` (document-generation) block, next to `listsExpanded_`, so opening,
reverting, restoring history, or otherwise replacing the document drops
path-keyed state. The `wasActive` guard in the click handlers additionally makes
the inactive-row rule independent of refresh timing.

### The delegate paints each selected bit

The bracket block draws the mask rect for bit 2, the vector rect for bit 4, and
the content rect for bit 1, keeping the existing group-icon suppression for the
content bracket. The drag/rename hit targets, outlines, and `singularActive`
guard are unchanged. The block is rewritten line-neutral to stay inside the
1200-line cap.

## Risks / Trade-offs

- The numeric `ActiveThumbRole` values change (content `0 -> 1`, raster
  `1 -> 2`, vector `2 -> 6`) -> update every assertion in the affected suites in
  the same change; the values are panel-internal and not persisted.
- A vector click now keeps the raster edit target -> `tst_panels_parity` asserts
  the white/black mask pair after the vector click and the image pair after the
  content-thumbnail click.
- A stale hash could show a vector on the wrong row -> pruned at the top of
  `refresh` for external and programmatic selection changes, and cleared in the
  `newDocument` block for document replacement and history restore. A structural
  renumbering that reuses the active anchor path can still carry a bit onto a
  different layer; that is pre-existing because layers have no stable id.
- The right-click raster menu still drops an active vector selection -> out of
  scope; it already behaved that way and the issue covers plain clicks.
- With thumbnails disabled no thumb is clickable; the target still normalizes,
  and no brackets are drawn.
