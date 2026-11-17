# Design: one stroke lifecycle

## Context

The live paint slot already has one owner (`ActiveSession::Stroke`). The begin and
end paths, however, each know a different subset of the session invariant:

- `impl_paint::begin_paint` resolves the layer or mask target, runs the memory
  gate, installs the session, clears the pending present, resets the tiles, opens
  recording, and then seeds the GPU or preview backend.
- `paint_tools::begin` builds a tool-specific stroke, installs the session, clears
  the pending present, resets the tiles, and opens recording.
- `impl_paint::begin_mask_paint` builds the mask stroke, installs the session,
  clears the pending present, resets the tiles, and leaves recording to its caller.
- `end_paint` destructures `StrokeSession` and forks on layer versus mask.

The target resolution, memory gate, and backend choice are genuinely per-path.
The slot, label, pad, fade capture, pending present, tile set, and recording are
not.

## Decisions

### One constructor with a built plan

`StrokeSession::begin` checks the slot kind, calls a builder closure that returns
a `StrokePlan`, and then owns the invariant. The plan carries the layer or mask
target plus the GPU stroke, dab placer, and preview the caller already selected.
Keeping backend selection in the caller preserves the existing rule that only the
paths that had GPU/preview continue to have it.

The builder closure runs only after the slot check, so a refused begin performs
no stroke construction and leaves the session, pending present, tiles, and
recording untouched.

### Typed finish result

`StrokeSession::finish` maps the session to `StrokeFinish`:

- `Layer(LayerStrokeFinish)` carries the stroke, preview log, label, structural
  base, pad, fade capture, tile set, and pending present.
- `Mask(MaskStrokeFinish)` carries the mask stroke, preview log, label, pad, and
  tile set.
- `None` is an empty session.

`end_paint` matches the result and delegates to the existing layer or mask commit
routine. The commit routines keep their current replay, write-back, history, and
recomposite order; only the data handoff changes.

## Risks

- The constructor must not change the order of the memory gate, slot check, and
  target validation. The builder closure keeps target validation where it was,
  and the memory gates stay in their existing callers.
- Tool recording must open exactly once. The constructor is now the only caller
  of `tool_recording::begin` on these paths; the per-tool callers drop theirs.
- The typed finish result must not move any replay or write-back step. The layer
  and mask routines are moved with their bodies unchanged.

## Verification

Rust unit tests cover the constructor invariants and the typed finish mapping.
The existing paint/eraser/healing/mixer Qt suites and the self-test paint checks
run unchanged; no golden baseline changes.
