# Design

## Context

`core` depends on no internal crate. `codec` and `adjust` depend on `core`.
`render` depends on `core`, `codec`, `adjust`, `filters`, `paint`, `ops`, and
`select`. `app` depends on `render` and `codec`. The row function pointers keep
that direction intact. `codec` owns PSD block construction, `render` owns the
catalogue, and `app` consumes render wrappers.

The taxonomy has 25 catalogue kinds and 22 `Adjustment` variants. `Auto` fans
out to three catalogue kinds. The three fills come from `FillContent`. Replace
Color and Match Color stay outside the catalogue.

Default blocks are heap values, so their builders cannot be const. Function
pointers to those builders can be const row fields. The six direct commands
construct unit or `Auto` values, so their constructors can be const functions.

## Goals

- Generate `AdjustmentKind` and `ALL` from one declaration.
- Put the dialog-default builder, layer-default builder, direct constructor,
  and GPU route on each catalogue row.
- Keep codec byte construction unchanged and keep render convenience wrappers
  for existing consumers.
- Derive WGSL markers from numeric ids and test inline and LUT ids together.
- Make GPU parity require every routed row instead of allowing silent skips.
- Preserve the deliberate difference between dialog and new-layer defaults.

## Non-goals

- Moving PSD encode/decode, layout, or apply code.
- Merging dialog and new-layer defaults.
- Adding HDR Toning, Match Color, or Replace Color to the catalogue.
- Changing shader math, dependencies, or menu order.
- Compile-checking the C++ dialog factory's runtime fallback.

## Decisions

### 1. `AdjustmentKind` stays in `pictura-core`

`Adjustment` and `FillContent` already live in core. Codec, render, and app can
all name the taxonomy without a dependency on `pictura-adjust`. Moving decode or
PSD encoding into `pictura-adjust` would add the wrong dependency direction.

### 2. One macro declares the enum and `ALL`

A local macro emits both `AdjustmentKind` and its `ALL` slice from one variant
list. `name`, `from_name`, `Adjustment::kind`, and `FillContent::kind` remain
exhaustive matches. A new variant therefore needs a name and a conversion, and
it automatically appears in `ALL`.

The catalogue is a slice without a fixed length. Its bidirectional test compares
all rows with `ALL`, so a missing row fails with the complete taxonomy.

### 3. Each row carries callable facts

`AdjustmentEntry` has these fields:

```rust
dialog_default: Option<fn(Defaults) -> AdjustmentData>,
layer_default: Option<fn(Defaults) -> AdjustmentData>,
direct: Option<fn() -> Adjustment>,
gpu: GpuRoute,
```

Codec exposes one builder per distinct default payload. Rows that share a
payload point both fields at the same builder. Rows with no such fact carry
`None`. This keeps Brightness/Contrast, Hue/Saturation, and Photo Filter layer
defaults distinct from their dialog defaults, and keeps Gradient Map's dialog
default sensitive to the toolbar colors.

Render keeps `dialog_default(kind, ctx)`, `layer_default(kind, ctx)`, and
`direct_adjustment(kind)` as wrappers that resolve the row and invoke its field.
Codec no longer has a kind-keyed default match. Core no longer has a direct-kind
match.

A row omission is caught by the catalogue test rather than by a fixed array
length. Exhaustive conversion and GPU parameter matches still make omitted
classifications fail to compile.

### 4. `GpuRoute::Inline` carries only an id

```rust
enum GpuRoute {
    None,
    Inline { id: u32 },
    Lut,
}
```

The route test derives `kind == {id}u` from each id and requires it exactly once
inside the shader's `fn adjust` body. It also obtains ids 6 and 7 from
`gpu::lut::kind` and runs the same uniqueness and WGSL checks. There is no
independent marker literal to drift from an id.

`inline_params` and `uses_lut` both resolve the route through the catalogue.
`inline_params` lists every unmodelled `Adjustment` variant explicitly instead
of ending in a wildcard. Hue/Saturation with color ranges still declines because
its parameter shape has no shader form.

The GPU-feature test `catalogue_rows_keep_every_fact_in_sync` iterates every
row. It checks default decoding and titles, direct adjustment kinds, route
eligibility, unique WGSL ids, and the catalogue-driven parity source. The parity
case uses each routed row's layer-default field, requires that field to exist,
and asserts `checked == expected`.

### 5. Kind resolution has one implementation

`Adjustment::kind` and `FillContent::kind` are the only block-to-kind
conversions. The preset store converts the typed kind to its id in one place.

Auto maps to `auto-tone`, `auto-contrast`, or `auto-color`. Those kinds have no
layer default and are not in the preset whitelist. Replace Color and Match Color
return `None`. Tests assert the Auto mapping, the preset refusals, and the
same-kind replacement guard.

### 6. Color Lookup keeps the `None` preset

The Color Lookup dialog and layer rows point at the same builder. It encodes the
identity cube with the name `None`, which resolves to preset index 0. The
committed fixture and its generator keep `Identity.CUBE` as an independent
decoding fixture. No PSD encoder or decoder behavior changes.

### 7. Direct constructors live in core and rows point at them

Core exports six const constructors: Invert, Desaturate, Equalize, Auto Tone,
Auto Contrast, and Auto Color. Each direct row stores the matching constructor.
Render's `direct_adjustment` wrapper preserves the existing app call path, and
the history label still comes from the row title.

## Risks

- Function-pointer rows do not force a new variant to receive a catalogue row.
  The generated `ALL` list and the bidirectional catalogue test catch that
  omission.
- The C++ dialog factory still ends in a generic runtime fallback. This change
  does not compile-check that site.
- GPU parity remains ignored without a suitable device. Its row count is now
  exact, but runtime results still need the GPU profile.
- The refactor moves default construction into hooks without changing payload
  bytes. Codec tests compare representative complete payloads from the former
  arms.
- Build, lint, formatter, OpenSpec, and repository checks were not run during
  this review follow-up. Tasks 7.2 through 7.6 record the remaining gates.
