# Proposal

## Why

Manual testing of #514 showed that after four brush strokes, jumping back to the
first `Brush` state left the fourth stroke visible. The layer was restored, but
the display cache kept the later strokes because the restore's damage was
bounded by crossed tile deltas and a stored composite whose delta could be
absent after a region refresh. A non-linear branch capture made it worse: the
appended state is a full anchor, so it has no delta whose tiles could describe
the pixels to rebuild.

## What Changes

- Include layer-channel and mask tile bounds in a history restore's damage even
  when the document has a stored composite, so a missing composite delta cannot
  leave the display stale.
- When a backward walk leaves a full anchor (a non-linear branch capture),
  report the restore as unbounded and rebuild the display in full.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/edit-history`: the restore damage covers layer tiles and a backward
  jump from a non-linear full anchor rebuilds the display.

## Impact

- `crates/pictura-app/src/history/damage.rs`: layer tiles always bound damage.
- `crates/pictura-app/src/history.rs`: a backward walk leaving a full anchor
  sets `Damage::everywhere`.
- Tests: `tst_history_options::jumpBackAcrossStrokesRefreshesTheDisplay`.
