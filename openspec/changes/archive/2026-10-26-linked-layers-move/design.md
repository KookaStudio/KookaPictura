# Design: Linked layers move together and show a link badge

## Link state is already in the engine

`PictureViewRust::link_sets` is a `HashMap<path, set id>` (transient session
state, design D9 of the archived `layers-panel-management` change). The bridge
already exposes `link_layers(paths, on)`, `select_linked(path)`, and a per-row
`linked` flag. This change only consumes that state; it adds no persistence.

## Row badge

`LayerRowDelegate::paint` walks the right edge from `itemRect.right() - 3`
through the lock badge, the fx badge, and the `Blend If` chip. A new
`showsLink(index)` / `linkBadgeRect(...)` pair draws `layers.link2` immediately
left of the fx badge (so the run is, from the right: lock, fx, link, Blend If).
`badgesRight` and `nameRect` subtract the same advance, so the mask/vector
thumbnails and the name rect keep their x. The glyph is omitted when the asset
is missing, matching every other badge.

## Move group

The Move tool's target is the single active layer (`layer_target.single()`).
When that layer is in a link set, the set's members are the move group:

- `move_group_paths()` returns the active path plus its set members in panel
  order (topmost-first) via `flatten_rows`; a single path when unlinked.
- `compute_move_preview(paths)` builds the base with every member hidden and the
  moving image as either the single layer's own image (unchanged path) or, for a
  set, `rendered_move_group` — the members composited over transparency in
  bottom-first stacking order, cropped to their union rect. The cache key is the
  members joined by `\n`; a set containing a shape layer is never cached (its
  image is rendered at the outline, which a move shifts without changing the
  rect).
- `translate_move_group(doc, paths, dx, dy)` translates each member through
  `translate_layer_path` and returns the union of every moved member's
  before/after rect. `commit_move`, `translate_layer`, and `move_preview` all
  use it, so the drag and the arrow-key nudge agree and record one "Move Layer"
  state.
- The live preview therefore shows the whole set moving; the commit
  recomposites the union dirty rect.

`ponytail:` a linked member whose blend mode reads the layers below previews as
if over transparency (the preview excludes the base from the set's render, as
the existing transform clipping-group preview does); the commit recomposites
correctly. A linked group is not translated (`translate_layer_path` refuses
groups), so linking a group and dragging a sibling leaves the group in place.

## Strip Link button

`syncStripButtons` enables the Link button when a document is open and the
panel has a layer selection. Its icon and click follow `linked_together(paths)`
— a new bridge query that is true when every selected path resolves to the same
non-zero link-set id. When true the button shows `layers.unlink2` and unlinks;
otherwise it shows `layers.link` and links. This mirrors the row menu's
Link/Unlink toggle and the `Layer > Link/Unlink Layers` commands.
