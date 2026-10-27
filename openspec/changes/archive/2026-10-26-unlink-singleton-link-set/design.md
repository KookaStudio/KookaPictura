# Design: Unlinking a member clears a two-layer link set

## Pruning singleton sets

`apply_link` mutates the `HashMap<path, set id>`. On unlink it now calls
`prune_singleton_links`, which counts members per set id and retains only sets
with two or more members. The unlinked layer's set entry is already gone; the
last remaining partner is dropped by the prune, so `layers_snapshot` reports
`linked: false` for it, the row badge disappears, and `select_linked` returns
empty.

Pruning runs only on unlink. Linking a single layer still creates its set
(the existing single-layer link behaviour, asserted by `lpr_row_fonts` and the
row-menu test); an unlink that severs the last partner drops the leftover, so
the two paths are not symmetric by design.

`changed` still counts the listed paths whose membership changed; the pruned
partner is not a listed path, so the return value is unchanged.
