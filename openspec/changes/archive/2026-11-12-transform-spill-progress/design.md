# Design

## D1. "Expected to spill"

`commit_cost` already splits a commit into `working` (resident: strips plus one
window a thread) and `spillable` (output tiles and the composite, which the
evictor may pack or spill). `admit_spilling` admits `working` against the room,
and `spillable` against the room plus the free scratch space. A commit whose
`working + spillable` exceeds the room can be admitted only by packing or
spilling tiles, so `memory_gate::expects_spill` returns true exactly then. With
no limit (`room() == None`) nothing is expected to spill.

The room is read once, on the GUI thread, after the gate. The thread-local test
pins (`set_available_memory_for_test`, `set_scratch_available_for_test`) apply
there, so a Qt test can force the decision without a large document.

## D2. Routing

`commit_transform` defers when `area >= DEFERRED_TRANSFORM_PIXELS ||
memory_gate::expects_spill(&cost)`. The deferred path already covers lifted
selections and linked members, and `runTransformCommit` already shows the
window and lands the job. Nothing changes in the engine or the C++.

## D3. Progress during spill I/O

The tiled resampler advances the job by each strip's rows after the strip
is written into tiles (`transform_tiled::resample`). A strip is at most 256
rows, so progress keeps advancing while the evictor packs and spills earlier
strips. A full-size check of this stays manual (see the test plan).
