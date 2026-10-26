# Design: The Move tool moves a multi-selection

## The move group

`move_group_paths` (transform session) no longer requires
`layer_target.single()`. It walks every selected path, expands each through
`link_members`, and keeps the members that are movable (`movable_layer`: a pixel
or shape layer), visible, and not position-locked. The survivors are ordered by
`flatten_rows` (panel order, topmost-first) so `rendered_move_group` composites
them bottom-first. `None` when the selection has no such member, so a hidden,
locked, group-only, or empty selection still does nothing.

The callers (`begin_move_preview`, `prepare_move_preview`, `commit_move`,
`translate_layer`, `move_preview`) drop the separate single-layer
visible/unlocked check: the group already excludes those layers. The drag
preview and the one `Move Layer` state cover the whole group, as they already
did for a link set.

## Interaction with links

Selecting one member of a link set still moves the whole set (the set is added
to the group). Selecting several layers moves all of them plus any linked
partner not itself selected, which is what the reported case — both linked
layers selected — needs. Alt-drag duplication still requires a single target
(`duplicate_move_target`), unchanged.
