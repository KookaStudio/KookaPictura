# Proposal

## Why

With **Sample All Layers** on, the Smudge tool reloads its carried patch from a
composite frozen at the press. The finger therefore never picks up the smear it
has just laid: the stroke stops dragging, replays press-time pixels, and trails
the pointer by the dab spacing instead of following what is on screen (#800).

## What Changes

- **Live pickup.** Each dab builds its carried patch from the press-time
  composite with the active layer's live pixels source-over'd on top. The other
  layers are static during a stroke, so this is the current composite for an
  opaque topmost target, and the live composite when smudging onto an empty
  topmost layer. Partial-alpha and mid-stack targets remain an approximation
  (recorded with a `ponytail:` comment).
- **No API change.** `Stroke::begin_smudge` keeps its `sampled` argument; the
  engine refreshes it per dab from the working layer it already writes.
- **Tests.** An engine test proves a Sample All Layers stroke picks up its own
  deposit, and a Qt Test case proves Sample All Layers smears as far as Sample
  All Layers off on a single-layer document.

## Capabilities

### Modified Capabilities

- `tools/smudge-tool`: Sample All Layers now reads the current composite rather
  than a press-time snapshot.

## Impact

- `crates/pictura-paint/src/smudge.rs`: the pickup patch composites the live
  working layer over the sampled surface, plus unit tests.
- `crates/pictura-app/cpp/tests/tst_retouch_tools.cpp`: the GUI regression case.
- Ceiling (`ponytail:`): a partial-alpha or mid-stack target double-counts the
  layer's press-time contribution under the live one; a full live composite
  would need the render layer inside the stroke.
