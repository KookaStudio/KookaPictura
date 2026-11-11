# Design: brush-dynamics-option-sets

## Model

`pictura-paint::dynamics` owns the vocabulary:

- `Control { Off, Fade(u16), PenPressure, PenTilt, StylusWheel, Rotation,
  InitialDirection, Direction }`.
- `DynamicsInput { pressure, tilt_x, tilt_y, rotation, wheel }`, all normalized
  to `0..=1` except tilt, which is degrees in `-90..=90`.
- `Dynamics` is a `Copy` block on `StrokeConfig`. It keeps the existing
  `scatter`, `count`, `size_jitter`, `angle_jitter`, and `roundness_jitter`
  fields (the archived `brush-dab-dynamics` contract) and adds the Control and
  the five sets.

A control signal is `0..=1`:

- `Off` → no control contribution.
- `Fade(n)` → `1 - dab_index / n`, clamped; `n = 0` is treated as `1`.
- `PenPressure` / `PenTilt` / `StylusWheel` / `Rotation` → the matching input.
- `InitialDirection` / `Direction` → the first / current tangent, used only by
  Angle.

The combine rule is the documented behavioural model, not an Adobe curve:
`value = lerp(min, base, signal)` then the jitter draw shrinks the result. With
Control `Off` and jitter 0 the value is exactly the old one. For Angle,
`Direction` and `InitialDirection` set the tip along the tangent and jitter adds
its random turn; other controls map the signal symmetrically around the base.

## Inputs

`Stroke` stores the last sample position, the first movement tangent, and a
per-stroke dab index. `sample_with` computes the segment tangent, builds the
effective `DynamicsInput` (Brush Pose overrides replace device values), and
passes it to the dab placer. `sample` calls `sample_with` with pressure only, so
every existing caller and every per-dab engine keeps its behaviour.

Color is drawn once per stroke by default (CS6) and per dab when `Apply Per Tip`
is on. The per-dab color, opacity factor, and flow factor travel in the placed
dab record; the raster's `Stencil` and `DabRaster` take them as arguments instead
of reading a stroke-constant paint. This keeps the GPU path unchanged: a stroke
with dynamics that change the per-dab paint declines the GPU as the pressure
overrides already do.

## Bridge and persistence

`PaintTip` gains a nested `PaintDynamics` bridge struct (plain `i32`/`bool`
fields; controls are indices). C++ `BrushDynamics` mirrors it with defaults, so
existing brace initializers keep compiling. `PaintTip` is built from
`PaintOptions` in one place (`paint_tip.h`), and `tip_config` translates it into
`StrokeConfig::dynamics`.

`BrushDynamics` serializes through the existing `PaintFields` table. Restore
goes through the same clamping setter as a UI edit.

## Panel

The left list keeps the existing `QListWidget`; each implemented row gets a
check state and a page in a `QStackedWidget`. A lock button beside the list
toggles the selected set's padlock (and the row shows a lock glyph). Disabled
sets retain their stored values. Texture and Dual Brush keep `NoItemFlags`.

Pages are built from `NumericField`, `QComboBox`, `QSpinBox`, and `QCheckBox`.
A shared Control row owns the combo and the Fade step spin; the spin is only
enabled for `Fade`. Every control has an object name for Qt Test.

## Padlocks

`BrushDynamics` carries one lock bit per set. The brush preset picker reads the
bits and skips the matching fields when it applies a preset; tip-shape padlocks
are out of scope for this change (the tip is not a dynamics set).

## Determinism

Every random draw still comes from `Stroke::rng` in the order the dabs are
placed. A control-only stroke consumes no random draws. Tests replay the same
stroke and compare pixels.

## Ceilings

- Behavioral parity only: Adobe's jitter×control curve, tilt scale, Brush
  Projection mapping, and pose ranges are inferred. Tests assert the documented
  model, not Adobe bytes.
- Texture and Dual Brush are the next changes; their rows remain disabled.
- The reduced-level preview and GPU stroke use the exact path only when the
  dynamics require per-dab paint; otherwise their existing behaviour stands.
