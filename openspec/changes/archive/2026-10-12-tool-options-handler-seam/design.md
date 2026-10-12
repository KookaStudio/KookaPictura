# Design: tool-options-handler-seam

## Target shape

```
tool_services.h   ToolServices: view/canvas/colors/drag/notify/annotations
                  (~24 methods, no option accessors)
tool_options.h    ToolOptions base + per-family groups:
                    explicit: PaintOptions, SelectionOptions, CropOptions,
                              CloneSourceOptions
                    generic:  ToolOption<T> for the existing option structs
tool_handler.h    ToolHandler: ToolServices& params + options() port
tools.{h,cpp}     ToolController: ToolServices + signal hub + resolver;
                  no option storage, no option setters
```

## Option groups

A group owns its family's state, clamps in its setter, and fires the callbacks
`ToolController` wires to its existing signals and handler hooks:

- `PaintOptions` (size, hardness, opacity, flow, mode, roundness, tip angle,
  spacing, flips, dynamics, auto-erase) — shared by every paint-family handler;
  `sizeChanged` drives `brushSizeChanged` and the hover ring, `tipChanged`
  drives `brushTipChanged`.
- `SelectionOptions` (combine mode, marquee style, feather, fixed ratio/size,
  tolerance, contiguous, anti-alias, sample-all-layers, magnetic
  width/contrast/frequency) — shared by the selection handlers;
  `magneticWidthChanged` drives `magneticWidthChanged`.
- `CropOptions` (ratio, delete-pixels, angle, pivot, classic, grid overlay,
  straighten mode) — shared by Crop and Perspective Crop; `optionsChanged`
  drives the handler hook and `cropOptionsChanged`, `angleChanged` the canvas
  straighten preview, `straightenChanged` the handler arming and cursor.
- `CloneSourceOptions` (five slots + active slot) — the Clone Stamp handler
  and the Clone Source panel; `changed` drives `cloneSourceChanged`.
- `ToolOption<T>` wraps each remaining option struct (Pen, Shape, Type,
  Gradient, Bucket, Stamp, Eraser, Background Erase, Magic Erase, Art History,
  Retouch, Tone, Eyedropper, Color Replace, Mixer, Healing, Patch,
  Content-Aware Move, Red Eye, Move). `set` clamps through `clampOptions(T)`
  and fires `changed`.

## Ownership

`ToolController`'s constructor creates each group (`std::shared_ptr`), wires
its callbacks, and passes it to the factories of the handlers that use it.
Handlers keep the `shared_ptr`, so the group lives exactly as long as its
family. The controller keeps non-owning pointers only for its own uses
(`resolveSelectionMode`, `adjustBrushSize`, `applyBrushShortcut`,
`setForeground`'s type-colour mirror). `ToolController::optionsFor<T>(id)`
resolves a typed group through `ToolRegistry::forTool(id)->options()`; that is
the port the options bar and panels use.

## Resolved couplings

- `onOptionsChanged` hooks (type/pen/shape/crop) are wired from the group
  callbacks, preserving today's "options changed → handler re-reads" behavior.
- `setCropAngle`'s pivot and canvas preview, `setCropStraightenMode`'s handler
  arming + cursor refresh, and `setEyedropperOptions`'s ring clear keep their
  side effects through dedicated callbacks.
- `setForeground` still mirrors the colour into the type options and the mixer
  reservoir; both live in their groups now.
- The `brushSizeChanged`, `brushTipChanged`, `cropOptionsChanged`,
  `typeOptionsChanged`, `shapeOptionsChanged`, `cloneSourceChanged`,
  `moveOptionsChanged`, `magneticWidthChanged`, and `mixerReservoirChanged`
  signals stay on `ToolController`, so every consumer and test connect is
  unchanged.

## File-size guard

`tools.cpp` (1198) must shrink: the 484-line setter block and the option
members leave. `tool_options.h` is a new file under the 800 target; the
options-bar files only swap setter expressions.

## Verification

- `cargo nextest run --workspace` (Rust untouched), `ctest -R '^tst_'` (78
  suites), both self-test invocations, `scripts/check-file-size.sh`,
  `scripts/check-selftest-budget.sh`, `scripts/guard.sh`.
- Behavior is unchanged by construction; the Qt suites are the oracle.
