# Design

## D1. One cost model for a whole-pixel translation

`move_commit_cost(dirty)` returns a `CommitCost` with the same composite terms
as `commit_cost` for a whole-pixel quad: `working` is the composite's strip
and group buffers (`3 · 4 · TILE · width`), `spillable` is the tiled region
composite (`4 · area`). A move does not resample, so it has no per-thread
source window and no new layer tiles. The strip term is shared with
`commit_cost` through `composite_strip_bytes`.

`commit_move` passes it to `memory_gate::admit_spilling` instead of
`admit(12 · area)`. A refusal still restores the document clone.

ponytail: under 16 Mpx the region composite stays dense and the sRGB copy is
still made. That is at most about 200 MB, uncharged, as it is for Free
Transform.

## D2. Refusal message

`moveMemoryRefusalMessage()` returns a scratch-disk message when
`edit_refused_for_scratch_full()` is set, mirroring
`transformMemoryRefusalMessage()`.
