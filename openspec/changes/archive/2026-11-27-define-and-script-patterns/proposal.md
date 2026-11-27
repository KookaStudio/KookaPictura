# Proposal

## Why

The pattern preset library can import tiles, but users cannot define one from an image or use the five CS6 scripted pattern layouts in Edit > Fill (#599).

## What Changes

- Define a named pattern from a hard rectangular selection, persist it in the current Patterns preset list, and make it available to supported pattern consumers, including Paint Bucket, Pattern Stamp, Pattern Fill layers and shape fills.
- Offer the five scripted layouts in the existing Fill dialog. Their placement algorithms are deterministic approximations, not Adobe algorithm or visual parity claims.
- Preserve supported imported .pat entries, existing ordinary fills and legacy index-based action replay; record portable snapshots for new patterned fills. Adobe .pat export remains outside this change.

## Capabilities

### New Capabilities

- `ui/pattern-authoring`: selection-based pattern creation and scripted pattern fills.

### Modified Capabilities

## Impact

Edit commands and Fill dialog, current preset library and pattern consumers, Rust fill engine and bridge, Qt and Rust tests. No new dependency.
