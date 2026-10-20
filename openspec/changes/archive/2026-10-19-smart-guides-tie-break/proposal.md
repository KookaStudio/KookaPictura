# Proposal: smart-guides-tie-break

## Why

The merged smart-guides spec contradicts itself: the alignment requirement says a
layer wins when a layer and the document are equally close, while the geometry
requirement and the original design note say the document wins. The solver
already implements layer-wins, but no scenario or test pins the tie down, so a
regression would go unnoticed. PR #757 review flagged this as a blocker.

## What Changes

- Correct the geometry requirement and the archived design note to the
  implemented rule: the earlier reference wins a tie, then the document.
- Add `tst_smart_guides::tiePrefersLayer`, which places a moving edge equally
  close to a document target and a reference target and asserts the reference
  wins.
- No production code change: `bestAxis` already considers references first and
  keeps the first candidate on an exact tie.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `tools/smart-guides`: the geometry requirement's tie rule is corrected and
  gains a scenario.

## Impact

- `openspec/specs/tools/smart-guides/spec.md` (via this change's delta).
- `openspec/changes/archive/2026-10-19-smart-alignment-guides/design.md` and
  its spec delta — the contradictory tie sentence is corrected.
- `crates/pictura-app/cpp/tests/tst_smart_guides.cpp` — the new test.
- No new dependencies.
