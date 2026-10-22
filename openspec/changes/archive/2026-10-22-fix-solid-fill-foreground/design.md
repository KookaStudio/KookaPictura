# Design

## Context

The app keeps the foreground colour in three places, and the fill-creation
paths read the wrong ones:

```
  toolbox swatch / Color panel / Swatches / Eyedropper
                          |
                          v
  +-------------------------------------------+
  | ColorState (QObject)                     |  the authoritative store
  |   foreground_ / background_               |
  +---------+-------------------------+-------+
            |                         |
   foregroundChanged           colorsChanged (the frame mirrors the
            |                  controller back into the state)
            v                         v
  +------------------+      +---------------------+
  | ToolController   |----->| Toolbox widget      |
  +------------------+      +---------------------+

  +-------------------------------------------+
  | PicturaMainWindow::foreground_            |  a third copy, written
  |   only by the alt-eyedropper              |  only by the eyedropper
  +----------------------+--------------------+
                         |
        frame_menus.cpp  Layer > New Fill Layer > Solid Color...
        layers_panel.cpp  the panel's Solid Color...  (hard-coded black)
```

The panel passes the constant `0xff000000`. The Layer-menu handler reads
`foregroundColor()`, which returned the eyedropper-only copy; that copy is an
invalid `QColor` until the eyedropper has sampled, so the handler's
`fg.isValid() ? fg.rgba() : 0xff000000u` fell back to black there too.

The engine is not involved: `add_solid_fill` already takes the colour it
fills with (`cxxqt_object.rs`), and the `SoCo` descriptor carries no alpha,
so an authored fill is opaque whatever the foreground's alpha is
(`create.rs`, `add_solid_fill`).

See proposal.md for the motivation.

## Goals / Non-Goals

**Goals:**

- A Solid Color fill layer carries the app's current foreground colour, from
  both creation surfaces.
- One authoritative store for the foreground colour, so the next caller cannot
  repeat this bug.

**Non-Goals:**

- The CS6 Solid Color picker: a New Layer dialog carrying name, colour, mode,
  and opacity (`docs/05-layers/fill-layers.md`). That needs a bridge command
  for a named, blend-moded, opacity-bearing fill layer, which this change does
  not add.
- Gradient fill colours, and pattern fills (tracked in #262).
- Making the foreground per-document, or making it part of the document
  session, the undo stack, or the bridge.
- Collapsing the `ToolController`'s copy of the foreground. It is the
  tool-facing mirror of `ColorState`, kept in step by the frame
  (`buildTools`); `Edit > Fill…` and the shape tool read it, and this change
  leaves that mirror alone.
- Giving the status-bar tool hint a requirement of its own. The hint's
  fallback text now carries the foreground because the frame's own copy is
  gone, and the change is recorded as a manual row in the test plan rather
  than as a new capability.

## Decisions

**The colour comes from the shared `ColorState`; no bridge field is added.**
The issue's fix direction proposed plumbing the toolbox colour into the Rust
bridge. Rejected: the bridge call already takes the colour, and the foreground
is app/session state, not document state. A bridge field would add a
synchronisation channel for a value the C++ side already owns, and would put
app state where document state belongs.

**The frame's `foreground_` copy is deleted, not synced.**
Three options: delete it and let `foregroundColor()` read the shared state;
keep it and drive it from `ColorState::foregroundChanged`; leave it alone.
Deleted: it is the copy that made the Layer-menu path black, and leaving a
stale public accessor named `foregroundColor()` invites the next caller to
repeat the bug. Syncing it would keep two copies of one value; leaving it
would keep a lie. The cost is that the status-bar tool hint's fallback text
now names the current foreground colour instead of only the last alt-eyedropper
sample.

**The panel takes the colour state by constructor injection.**
`LayersPanel(ColorState* colors, QWidget* parent)` matches `ColorPanel` and
`SwatchesPanel`, which already receive the same object. A provider callback
would be more flexible than the single known source, and would need a second
concept for "no state yet". The `ColorState*` parameter defaults to `nullptr`
so the two bare constructions — `selftest.cpp`'s
`new pictura::LayersPanel()` and `tst_layers_panel_fixes.cpp`'s stack-allocated
`pictura::LayersPanel panel;` — keep compiling unchanged; a panel with no
colour state falls back to black, which is the default foreground anyway.

**The colour is read when the command runs, not when the panel is built.**
The frame builds the panel before the user has chosen anything, so a colour
captured at construction would be the default. The panel therefore holds the
state, not the colour.

**The tests go into the suites that already own the behaviour.**
`tst_layers_panel.cpp` is at 1399 lines against the 1400-line test cap
(`scripts/check-file-size.sh`), so it cannot take another case; the panel case
goes to `tst_layers_panel_fixes.cpp`, and the Layer-menu case to
`tst_layer_commands.cpp`, which already dispatches Layer-menu commands
through the registry. Both need `#include "panels/color_panel.h"`: `frame.h`
only forward-declares `ColorState`, so the type is incomplete without it.

## Risks / Trade-offs

- [The status-bar tool hint's fallback text now names the current foreground
  colour instead of only the last alt-eyedropper sample] → The change is
  confined to the fallback text; the keycap hints are untouched, and no test
  asserts the old text. The fallback is painted only for tools without
  shortcut hints, so the readout shows for the Eyedropper, the Ruler, the
  Note, the Count, the Zoom, the Hand, and the Paint Bucket, and not for the
  default Move or the Brush, which show keycaps instead; the bar is also
  hidden without an open document. The hint is refreshed from
  `ColorState::foregroundChanged`, connected in `buildTools` next to the
  other `ColorState` connects, so it follows the colour rather than going
  stale. Recorded as a manual row in the test plan.
- [The frame swaps the foreground with the background when the edit target
  becomes a mask] → The swap happens in `buildPanels`'s `editTargetChanged`
  connect, so a fill created while a mask is the target takes the mask's
  foreground (white by default). That is CS6's behaviour too — the foreground
  is whatever the current target's pair says — and the `scf_default_black`
  scenario pins the colours explicitly, so the behaviour stays well-defined.
  The new tests set the colour last and re-assert it, so a swap shows up as a
  clear failure rather than a wrong colour.
- [The shape tool reads the controller's foreground while the fill paths read
  the shared state's] → The frame keeps the two in step (`buildTools`), so
  they agree; the shape path is unchanged.
- [The frame's own `foregroundColor()` accessor now answers from the shared
  state, which the alt-eyedropper self-test reads]
  → The eyedropper writes that same shared state, so the check still sees the
  sampled colour.
- [The panel is given a colour state and could start caching the colour
  instead of reading it per command] → The panel holds the state, not the
  colour: the action reads `colors_->foreground()` when it fires, and
  `scf_panel_foreground` sets the colour after the panel already exists, so a
  cached colour would fail the case.
