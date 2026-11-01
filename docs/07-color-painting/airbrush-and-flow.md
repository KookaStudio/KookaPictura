# Airbrush and Flow

- **Spec ID:** `BRU-005`
- **Status:** `Draft`
- **Parity tier:** `Core` — Flow, Opacity, the Airbrush option, and CS6 airbrush tips are all in CS6 Standard.
- **New in CS6:** `Changed` — the **Airbrush tip** category is new in CS6 (Distortion, Granularity, Spatter Size/Amount); the pre-existing **Airbrush option** (time-based build-up) is unchanged.
- **Depends on:** `BRU-001` brush-engine, `BRU-002` brush-dynamics, `BRU-003` bristle-brushes, `BRU-004` mixer-brush-engine, `03-tools/brush-and-pencil.md`.

> Module and widget names are **design proposals**. The repository contains a
> working implementation; this file stays the behavioural contract for the
> feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped and what
> did not. Adobe's exact accumulation formula is closed; this spec is **behavioral
> parity only, algorithm TBD** except where the Help states a rule (notably the
> opacity/flow relationship).

## CS6 behavior

Three distinct concepts are often conflated:

1. **Opacity** — the per-stroke transparency cap. Help: "As you paint over an
   area, the opacity does not exceed the set level no matter how many times you
   move the pointer over the area, until you release the mouse button. If you
   stroke over the area again, you apply additional color, equivalent to the set
   opacity. Opacity of 100 percent is opaque."
2. **Flow** — the per-dab application rate. Help: "Flow Sets the rate at which
   color is applied as you move the pointer over an area. As you paint over an
   area, keeping the mouse button down, the amount of color builds up based on
   the flow rate, up to the opacity setting. For example, if you set the opacity
   to 33% and the flow to 33%, each time you move over an area, its color moves
   33% toward the brush color. The total will not exceed 33% opacity unless you
   release the mouse button and stroke over the area again."
3. **Airbrush** — a time-based build-up mode. Help: "Simulates painting with an
   airbrush. As you move the pointer over an area, paint builds up as you hold
   down the mouse button. Brush hardness, opacity, and flow options control how
   fast and how much the paint is applied." The Brush panel's **Airbrush/Build-up**
   checkbox is the same flag as the options-bar Airbrush button.

Practical consequences:

- With Airbrush off, holding the pointer still does not add paint; coverage comes
  from the pointer *moving* over new dabs.
- With Airbrush on, holding still keeps adding paint at a rate governed by Flow
  (and Hardness), up to the Opacity cap. The Wacom guide adds that build-up also
  affects scatter brushes: they "keep dropping tip stamps for as long as the pen's
  held down."
- Flow and Opacity interact through **dab overlap**. If Spacing = 100% (no
  overlap), a single pass with Flow = 10% and Opacity = 100% is equivalent to
  Flow = 100% and Opacity = 10%; overlap is what makes Flow build within one
  stroke. This is the standard community explanation and matches the Help's
  "each time you move over an area" wording.

**CS6 Airbrush tip** (a tip type, not the mode): Help describes it as replicating
spray cans with a 3D conical spray; Size, Hardness, Distortion, Granularity,
Spatter Size, Spatter Amount, and Spacing are settable, and "with a stylus, you
can alter the spread of sprayed strokes by changing pen pressure." This is
distinct from the Airbrush *option* (build-up) and from the legacy soft "airbrush
brush" preset.

**Keyboard:** `Shift+Alt+P` toggles the Airbrush option.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Options bar | Toggle | `Shift+Alt+P` | Airbrush button |
| Options bar | Slider | — | Opacity |
| Options bar | Slider | — | Flow |
| Brush panel | Checkbox | — | Airbrush/Build-up (same flag) |
| Brush panel > Brush Tip Shape (Airbrush tip) | Options | — | Size, Hardness, Distortion, Granularity, Spatter Size, Spatter Amount, Spacing |
| Keyboard | — | number keys / `Shift`+number | Opacity / Flow in 10% increments |
| Keyboard | — | `00` (Mixer) | Wet and Mix to zero (`BRU-004`) |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Opacity | int % | 100 | 0–100 | Per-stroke cap |
| Flow | int % | 100 *(unverified)* | 0–100 | Per-dab rate; shorthand `Shift`+numbers |
| Airbrush / Build-up | checkbox | off | on/off | Time-based accumulation |
| Airbrush tip Size | int px | preset | 1–5000 | Tip type, not the mode |
| Airbrush tip Hardness | int % | preset | 0–100 | Hard-center size of the cone |
| Airbrush tip Distortion | int % | preset | 0–100 *(unverified)* | Distorts the spray |
| Airbrush tip Granularity | int % | preset | 0–100 *(unverified)* | Graininess of the drops |
| Airbrush tip Spatter Size | int | preset | 0–… *(unverified)* | Size of paint droplets |
| Airbrush tip Spatter Amount | int | preset | 0–… *(unverified)* | Number of droplets |
| Airbrush tip Spacing | int % + checkbox | preset | 0–1000% *(unverified)* | Droplet spacing; unchecked = pointer speed |

## Algorithms & pipeline

**Behavioral model (exact Adobe formula TBD).**

Per-pixel compositing for a dab of color `B` over canvas `C`, with dab coverage
`a` and effective opacity cap `O`:

- Without airbrush: within one stroke, track per-pixel accumulated coverage
  `acc` and clamp it to `O`. New coverage from a dab at rate `flow` gives
  `acc = min(O, acc + flow · a · (1 − acc))` (order-dependent; exact Adobe
  operator TBD). Releasing the stroke commits and resets `acc` to 0, so a new
  stroke can add another `O` step.
- With airbrush: while the pointer is held, dabs continue to be emitted at a
  time-driven rate even when the pointer is stationary, so `acc` keeps rising
  toward `O`. This is the only difference from the non-airbrush path.
- Flow is a **per-dab** multiplier; Opacity is a **per-stroke** cap. Their
  perceived equivalence at Spacing = 100% follows directly from there being no
  overlap to accumulate.
- Airbrush tip: a cone/spray generator rather than a hard round mask. Granularity
  perturbs droplet alpha, Spatter Size/Amount set droplet size/count, Distortion
  warps the spray direction, Hardness sets the core, and pen pressure widens or
  narrows the spray spread.
- **Interaction with Spacing:** lower spacing ⇒ more overlapping dabs per unit
  path ⇒ faster accumulation at a given Flow. Very low spacing + airbrush can
  reach the cap almost immediately; high spacing approaches the non-overlapping
  equivalence.
- **Interaction with Pressure:** with Transfer Flow/Opacity jitter
  (`BRU-002`), pen pressure scales flow/opacity per dab, so pressure changes both
  the rate and the streak density.
- **Scatter + airbrush:** with Scatter enabled and Airbrush on, the brush keeps
  dropping scatters while held (Wacom guide), so the accumulation is spatial as
  well as temporal.

Proposed dab cadence: a time-based emitter feeding the same dab pipeline as path
interpolation, sharing the stroke-scoped coverage accumulator and RNG seed so
redo is deterministic.

## Rust module mapping

- `pictura-brush::flow::StrokeAccumulator` — per-pixel `acc` buffer with the
  Opacity cap; reset on stroke release.
- `pictura-brush::flow::FlowConfig` — `{ opacity, flow, airbrush: bool }`.
- `pictura-brush::airbrush::AirbrushTip` — `{ size, hardness, distortion,
  granularity, spatter_size, spatter_amount, spacing }`; cone spray generator.
- `pictura-brush::engine::DabEmitter` — merges path-driven and time-driven dab
  emission when airbrush is on.
- `pictura-core::command::BrushStroke` — as `BRU-001`.

Boundary types: `FlowConfig`, `AirbrushTip`, `Dab`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `PaintOptionsBar` | `QWidget` | Opacity, Flow, Airbrush toggle, Mode |
| `AirbrushTipEditor` | `QWidget` | Size/Hardness/Distortion/Granularity/Spatter Size/Spatter Amount/Spacing |
| `AirbrushIndicator` | `QToolButton` | Reflects and toggles the shared Airbrush/Build-up flag |

Widgets. The Brush panel checkbox and options-bar button must be two views of one
model flag, not two states.

## Data-model impact

- **No PSD fields.** Flow/Opacity/Airbrush are tool or preset state.
- **Preset serialization:** `flow` and `opacity` are descriptor values; the
  Airbrush/Build-up flag maps to the `usePaintDynamics`/build-up area of the
  `.abr` descriptor (community ABR mapping is incomplete — `BRU-006`).
- **Undo:** black-boxed by `BrushStroke`; the per-stroke accumulator is scratch,
  discarded on commit.

## Edge cases

- **Airbrush on + stationary pointer.** Must keep emitting at a bounded rate; no
  infinite loop on a still pointer and no unbounded CPU at max Flow.
- **Flow = 0.** No paint regardless of hold time.
- **Opacity = 0.** No paint (short-circuit).
- **Spacing unchecked.** Time-driven emission must remain sane; velocity-driven
  spacing plus airbrush must not double-count.
- **Airbrush tip + Airbrush option.** The tip type and the build-up mode are
  independent; both may be on.
- **Scatter + airbrush.** Cap dab emission per frame.
- **8/16/32 bpc and CMYK/Lab.** Accumulation math is color-model-specific;
  Indexed/Bitmap unsupported.
- **GPU unavailable.** CPU accumulation; previews degrade.
- **Undo mid-stroke.** Atomic on release; cancel discards the accumulator.

## Parity acceptance criteria

- Given Opacity = 100%, Flow = 10%, Spacing = 100%, a single pass produces ~10%
  coverage; repeated passes with the button held do not exceed 100%; releasing
  and re-stroking adds another step.
- Given Opacity = 33%, Flow = 33% with overlapping dabs, coverage per stroke does
  not exceed 33%; a second stroke can raise it toward 100%.
- Given Airbrush on, holding the pointer stationary increases coverage toward the
  Opacity cap; with Airbrush off it does not.
- Given Flow = 0, no paint is applied.
- Given a lower Spacing at fixed Flow, coverage reaches the cap faster (more
  overlap).
- Given an Airbrush tip, a stroke shows granular multi-droplet spray and pen
  pressure changes the spray spread.
- Given Scatter + Airbrush, marks continue to be emitted while the pointer is
  held still.
- Given `Shift+Alt+P`, the Airbrush option toggles and the Brush panel checkbox
  reflects the same state.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  Adobe Photoshop Help reference (downloaded and text-extracted). Established:
  Opacity as a per-stroke cap; Flow as a build-up rate up to opacity with the
  opacity=33%/flow=33% worked example; Airbrush as time-based build-up governed
  by hardness/opacity/flow, and the Brush panel Airbrush/Build-up equivalence;
  `Shift+Alt+P`; the CS6 Airbrush tip options (Size, Hardness, Distortion,
  Granularity, Spatter Size, Spatter Amount, Spacing) and pen-pressure spread;
  numeric opacity/flow shortcuts. Primary source.
- `https://community.wacom.com/en-co/complete-guide-to-photoshop-brushes-pt-3` —
  Wacom brush guide. Established: "Enable Airbrush Style Build-Up Effects" is the
  duplicate of the options-bar Airbrush option; it also keeps scatter brushes
  dropping stamps while held; Spacing at 1% vs 100% and its effect on stroke
  continuity. Secondary.
- `https://glensmith.co.uk/photoshop/mixer-brush` — Mixer walkthrough.
  Established: airbrush-style build-up is an option on painting tools and how it
  is described to users. Secondary.

## Open questions

- **Accumulation operator.** Whether within a stroke Adobe uses linear addition
  clamped to opacity, `src-over` with a running alpha, or a coverage-normalized
  blend is undocumented — algorithm TBD. Resolve by measuring coverage on a
  single-pixel target across controlled flow/spacing.
- **Default Flow value.** The Help does not state it; marked *(unverified)*.
- **Airbrush emission rate.** The time unit/rate of stationary dab emission is
  undocumented. The stamp tools use 20 dabs per second as an approximation.
- **Airbrush tip ranges and defaults.** Distortion/Granularity/Spatter ranges and
  defaults are not stated; marked *(unverified)*.
- **Airbrush tip vs Airbrush option UI.** Whether selecting an airbrush tip
  implies the build-up option or they are fully independent needs confirmation.
- **Scatter + airbrush interaction.** Wacom's modern note may not exactly match
  CS6; verify.
- **CMYK/Lab accumulation.** Whether flow/opacity operate on device channels or
  the working space is unverified.
