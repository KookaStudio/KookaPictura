# Design: Real editing tools

## Parameters

`filter` checks values in C++ against `FilterCommandSpec`, so the client gets a
precise `invalid_param`. The engine's own range check stays underneath. A filter
with a Hidden control or `mapAfterOk` needs a resource and is refused.

## run_command

Replay runs `executeRecorded`, which opens no UI. The table lists each id with
the parameter keys it may carry, and an unknown key is `invalid_param`, so a key
such as `in` on Save can't bypass the path gate. Save and every `file.*` id stay
out. Image Size and Canvas Size are checked against `ImageBudget` first.

## Layers

`list_layers` keeps its top-level `index`, so existing clients are unaffected,
and adds `active`. `layer_tree` returns every row with its path and depth.

## Undo

Each call is one history state; `lasso` keeps that true for a selection. There is
no transaction API, and `history` `jump` or a snapshot gives an agent its rollback.

## Stale guards

`documentInfo` reports an `id`. A mutator carrying `doc` is answered
`stale_document` when another document is active, because the person may have
switched tabs. `expected_index` on `edit` answers `stale_history`.

## Crops

The human-busy gate refused every request during a pending crop, including the
agent's own. The server now notes when a `pointer` request created the crop. Only
then do `pointer` and `commit_crop` pass the gate.
