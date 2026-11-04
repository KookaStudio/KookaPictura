# Design

## When the fallbacks are registered

The compositor lays out type layers itself (`layer_source`), keyed on the font
registry generation, so the fallbacks matter to any document holding type, not
only to the Type tool. Two triggers cover that:

- **The debounced refresh**, when `type_api::hasTypeLayers` reports a type layer
  in the active document. This keeps today's timing for a document with type:
  the faces were registered on the first refresh after open before, and still
  are. `has_type_layers` walks the layer tree and stops at the first type layer,
  so the check costs no pixel work.
- **Every `type_api` call that lays out text** (previews, metrics, commits,
  updates, styles, conversion, warp, an edit session, the gamma and
  glyph-protection refreshes, Replace All Missing Fonts). This covers type set
  in a document that had none. After the first scan, each call is a flag check.

The control server's type `add` dropped its own call because `commitLayer`
now makes it.

ponytail: type inside a smart object's embedded document is not seen by
`has_type_layers`; it lays out with the faces registered so far, as it did
before the first refresh.

The once-only flag moved into a function so `forgetTypeFallbacksForTest()` can
reset it. Faces already handed to the engine stay registered, so a reset only
re-runs the scan.

## Agent Control provider

`AgentControl` is constructed with the frame as its parent (in `main.cpp` and
in the tests), so `findChild(…, Qt::FindDirectChildrenOnly)` finds it without
walking the panels and their widgets. `frame.h` is at its 1200-line cap, so a
cached pointer member was not added.

## Measured after

| Debounced refresh | After |
|---|---|
| First run after open (retarget) | 7 ms: Layers 3.0, Channels 1.8, Properties 0.9, every other panel under 0.6 |
| Every later run | 2–4 ms |

The Agent Control provider no longer shows in the per-command registry timing.
