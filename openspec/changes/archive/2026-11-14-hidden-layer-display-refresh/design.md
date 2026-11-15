# Design

## D1. Why the requested paths are the wrong region

`pictura_render::apply_visibility` sets exactly the listed paths visible and
hides every other node. The app's `apply_visibility` used `batch_changed`, which
bounds the refresh by the influence of the **listed** paths. For solo the
listed paths are the ones kept visible; the nodes that actually change are the
unlisted ones. Their pixels lie outside the computed rectangle, so the display
tiles kept the old picture there.

## D2. Compute the changed set first

`visibility_change_region` walks the flattened tree before the mutation and
unions `damage::influence` for every node whose visibility differs from the
requested visible set. An unbounded changed node (an unmasked adjustment, a
group with no bounded reach, a knockout document) returns `None`, which falls
back to `Edit::Full` exactly as the other bounded edits do.

The walk uses the same `flatten_rows` + `resolve_path` pair as the solo snapshot,
so it adds no new traversal shape. It runs only for the solo and Show All Layers
commands, not for the ordinary eye toggle (which keeps the existing per-path
`set_visible_paths_union`).

## D3. Present levels and the merged preview

The region edit goes through `refresh_display_rect`, so the damage reaches every
cached pyramid level and the lazily opened file's merged preview is dropped
before the composite is rebuilt. The fit-level present therefore matches a full
recomposite; the Qt tests assert that byte for byte.

## D4. Restore

`apply_visibility` also serves `solo_restore`. Restoring the snapshot flips the
solo-hidden nodes back on, so the same changed-set walk covers the restore with
no separate code path.
