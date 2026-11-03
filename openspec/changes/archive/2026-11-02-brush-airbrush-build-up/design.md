# Design

## One flag, two views

The Brush's Airbrush state lives in `PaintOptions` beside Opacity and Flow, so
the options bar, the Brush panel, and the shortcut all read and write the same
value. `setAirbrush` emits the existing `paintSettingsChanged` signal; both views
listen to it and re-read the model with their own signals blocked, so a change
from either view (or the shortcut) reaches the other without a feedback loop.

The Pencil shares the `PaintOptions` object but shows no Airbrush toggle and
ignores `Shift+Alt+P`, matching `docs/03-tools/brush-and-pencil.md` ("for the
Brush tool an Airbrush toggle").

The stamp tools keep their per-tool flags in `StampOptions`. `StampGroup::changed`
now emits `stampOptionsChanged`, so a shortcut toggle updates the stamp options
bar the same way.

## Held dabs

`DabPlacer::feed` places nothing on a zero-length segment, so re-feeding the
pointer position cannot paint. `DabPlacer::repeat_last` (from #799) returns the
last sample without touching the carried residual, so path spacing after a hold
is unchanged. The Brush handler uses the same 50 ms `QTimer` as the stamp tools;
a timer does not queue ticks, so slow dabs lower the rate and never stack.

## Every Brush backend

`paint_hold` mirrors `paint_dab`'s routing:

- **Mask stroke:** `mask_hold` holds the mask document's exact stroke, writes the
  dirty region back, and presents it, exactly as `mask_dab` does.
- **GPU stroke:** `gpu_hold` asks the GPU placer for the last dab and applies it
  through the same shared `gpu_dabs` step as `gpu_dab`.
- **Preview stroke:** `preview_hold` appends a held entry to the preview log and
  rasterizes at the preview level. The log becomes a sequence of moves and holds,
  so the commit replay can call `Stroke::hold` for the held entries. Without that,
  the preview would build up on screen and the committed pixels would drop the
  build-up.
- **Exact CPU stroke:** `Stroke::hold`, as for the stamps.

A mask GPU stroke routes through the same shared `mask_gpu_dabs` step.

## Shortcut

`Shift+Alt+P` is a window `QShortcut`, like the `[` / `]` brush keys, because it
acts on the active tool rather than a menu command. `ToolController::toggleAirbrush`
owns the tool dispatch. `workspaceKeys()` lists the binding so the keyboard
shortcuts editor cannot assign the same key to a command and the
`workspaceKeysMatchTheWindow` guard keeps passing.

## Recording

Held dabs stay out of the recorded-action log, as with #799: an action replays
the path only, and the build-up depends on time held. This is a named ceiling.
