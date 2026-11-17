# Proposal

## Why

The CS6 Extended image-stack Stack Modes are specified in
`docs/05-layers/smart-objects.md` (the modes, formulas, non-cumulative render,
and ≥2-layer rule) but absent from the codebase: the docs↔code parity register
records LAY-13 as `MISSING` with "no `StackMode`/`reduce_stack`", and
`Layer > Smart Objects > Stack Mode` is a disabled placeholder row. A placed or
edited multi-layer Smart Object therefore cannot be reduced non-destructively.

## What Changes

- Add `StackMode` (Maximum, Minimum, Mean, Median, Summation, Range, Variance,
  Standard Deviation, Skewness, Kurtosis, Entropy) and `reduce_stack` to
  `pictura-core`, and carry the mode as
  `SmartObject.stack_mode: Option<StackMode>`.
- Render an active mode from the embedded payload's layers, per channel over
  non-transparent samples, ignoring the raster proxy; every render re-reads the
  payload, so switching modes is non-cumulative.
- Add the engine operation `set_smart_object_stack_mode`, which refuses an
  ineligible object (fewer than two frames, no embedded payload, not a smart
  object) without mutating, and clears the mode for `None`.
- Make `rasterize_smart_object` bake the reduced stack render when a mode is
  active, even when the layer carries a raster proxy.
- Persist the mode in a Pictura-private layer block that round-trips through
  `write_psd`/`read_psd`; the layer's raster proxy stays the pre-stack content.
- Add the 12 Extended-only `Layer > Smart Objects > Stack Mode` commands
  (`None` + the 11 modes), checkable, enabled only for eligible objects, each
  recording exactly one history state.
- Update the parity register (`LAY-13`), `ROADMAP.md`, and `docs/dev/STATE.md`.

**Non-goals:** multi-layer `Convert to Smart Object`, `File > Scripts >
Statistics` / `Load Files into Stack`, the inferred Properties-panel Smart
Object page, linked smart objects, and reading Photoshop's undocumented
descriptor key for a stack mode it authored.

## Capabilities

### New Capabilities

- `compositing/smart-object-stack-modes`: the `StackMode` model and reduction
  math, frame and sample-set semantics, apply/clear eligibility, source-preferred
  rendering, rasterize behaviour, and the Extended-only command surface.

### Modified Capabilities

- `compositing/smart-object-rendering`: the proxy-precedence requirement gains
  the active-stack-mode exception, and the nearest-neighbour placement
  requirement accepts the reduced stack source.
- `compositing/smart-object-layer-actions`: rasterizing materializes the reduced
  stack source when an eligible active stack mode applies; otherwise a proxied
  layer keeps its proxy.
- `compositing/layer-source-cache`: changing a smart object's stack mode
  invalidates its cached source and composited raster.
- `codec/psd-smart-objects`: the Pictura-private stack-mode marker round-trips.
- `ui/edition-gating`: the stack-mode commands join the Extended-only command
  set.

## Impact

- `crates/pictura-core`: new `StackMode`/`reduce_stack` module, one `SmartObject`
  field and its equality.
- `crates/pictura-render`: source rendering, content route, cache key, rasterize,
  and the new apply/eligibility operations.
- `crates/pictura-codec`: private marker block read/write.
- `crates/pictura-app`: cxx-qt bridge, command catalogue, const names, the
  generated `command_ids.h`, and the Smart Objects menu wiring.
- Tests: core unit tests per mode, render integration tests, codec round-trip,
  and Qt Test coverage for the command surface and edition gating.
- Docs: parity register, roadmap, state notes (separate `docs:` commit with
  `TASK-ALLOWS-DOCS`).
