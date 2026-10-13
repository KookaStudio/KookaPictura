# Design

## Context

The six fixes touch the Layers panel delegate, the panel's edit-target wiring,
the mask bridge, and the layer-move bridge. Each is small; this records the
non-obvious decisions.

## Decisions

### Opacity label alignment

`NumericField` already lays the leading label out in the same `QHBoxLayout` as
the fixed-width value box and the arrow, so the label absorbs the row's slack.
The Opacity field therefore has an ~88 px label with its text at the left. A new
`NumericField::setLabelAlignment` sets the label's text alignment; the Layers
panel right-aligns the Opacity label (`Qt::AlignRight | Qt::AlignVCenter`),
which pushes its text against the value box. The Fill label has no slack (its
field ends the lock row), so it is left alone.

### Mask add activates the mask

CS6 selects a freshly added mask so the brush/fill/filter write coverage at once.
`layer_mask_add_path` now sets the view's `mask_edit_target` on a successful add
(a refused add leaves it alone). The panel's delegate brackets are driven by the
panel-local `ActiveThumbRole`; rather than duplicate the target in a second
place, `LayersPanel::refresh` derives the active thumbnail from the bridge's mask
target (`1` when the row is the target, otherwise the stored thumb, with a stale
raster target falling back to the image). Clicking a vector-mask thumbnail now
clears the raster target so the two indicators cannot disagree. This changes two
existing tests that assumed the target was only set on click
(`tst_layers_panel::maskEditTargetActivation`, `tst_mask_edit::colorsFollowTheEditTarget`);
both are updated to the new behaviour.

### Group expansion survives a structural change

Expansion is tracked by index-based path, and adding, removing, or moving a layer
can shift a group's own index when the source leaves or enters its container.
The model gains `namePathForPath` (an ancestor-name chain key) and
`indexForNamePath`. Before the model reset `LayersPanel::refresh` captures each
expanded path's name chain; after the rebuild it keeps the path when it still
names the same chain, else falls back to the name chain, so a shifted group is
re-found and stays expanded. A rename leaves the index path valid, so the primary
lookup keeps its state. This generalises the earlier drop-into fix, which is
removed (along with its engine/bridge change).

### Thumbnail geometry

The thumbnail box was a `thumb × thumb` square, so a tall document's thumbnail
was capped at `thumb` and left vertical padding. The box is now `thumb` wide and
the row's full height minus a 2 px pad (`thumbSlot`), and the letterbox fits the
document aspect inside that box. `letterboxedThumb`'s math was wrong for a
non-square box (it used `box.height()` when fitting by width and vice versa); it
is corrected to fit by width or height as the box aspect requires. The slot width
is unchanged, so the name position and hit-targets are unaffected.

### Chain glyph

The link/unlink icons are drawn at `linkSide(thumb) + 2` and passed through
`QTransform().rotate(90)`; the target rect is centered on the unchanged slot.
Both linked (`layers.link2`) and unlinked (`layers.unlink2`) states use the same
helper. A group's folder glyph keeps no outline: the 1 px black outline is
skipped for `kind == "group"`.

### Folder glyph size

The group folder glyph grows from `thumb / 2` to `thumb * 3 / 4`
(`LayerRowDelegate::groupIconSide`), so it reads close to a layer thumbnail in
size while staying centred in the slot.

### Combo affordances

The global stylesheet styled `QComboBox::drop-down` but declared no
`::down-arrow`, so the arrow read as missing. Qt's stylesheet engine cannot draw
the design system's CSS triangle (it renders the border box as a square), and a
`currentColor` SVG renders black with no palette tint, so the caret ships as a
fixed-colour asset (`assets/ui/combo-caret.svg`, plus a disabled variant) in the
UI text and disabled-text colours, both constant across the four brightness
levels. Up/Down cycling a focused non-editable `QComboBox` is native Qt
behaviour; the real defect was that the panel's `syncControls` blanket-disabled
the header controls and re-enabled them, and disabling a focused widget drops
its focus. It now computes the final enabled state once, so a control that stays
enabled keeps focus and repeated arrow keys keep cycling.

### Blend-change history label

The blend bridge labelled every change `Blend Mode`. `BlendMode::name` in
`pictura-core` returns the CS6 display name (the same strings as the app's
`blend_modes.h` table), and both blend bridge entry points label their undo state
`Blending Change: <name>`.

## Alternatives

- Keeping the square thumbnail box and shrinking the row was rejected: the
  "Raised layer row height" requirement mandates a ≥ 34 px Medium row, so the
  square thumbnail cannot fill it.
- Computing a shifted group's new path from the move was rejected in favour of
  the name-chain lookup, which also covers adds and removals and keeps the panel
  free of engine post-removal math.
- A second panel-local raster target was rejected in favour of deriving the
  active thumbnail from the bridge target, which keeps one source of truth.
