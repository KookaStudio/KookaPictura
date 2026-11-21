# Tasks

## 1. Tests first

- [x] 1.1 Add `pictura-paint` tests for midpoint bias, opacity stops, and
  reverse.
- [x] 1.2 Add `tst_image_adjustments` cases for selecting, editing, adding,
  and removing opacity stops and midpoints.
- [x] 1.3 Add `tst_fill_tools` cases for opening the editor from the sample,
  applying and cancelling an edited ramp, and returning to a preset.

## 2. Engine

- [x] 2.1 Add `midpoint` to `GradientStop` and an opacity-stop list to
  `Gradient`; sample colour and opacity independently with the midpoint bias.
- [x] 2.2 Reverse and grayscale paths preserve midpoints and opacity stops.

## 3. Editor model and UI

- [x] 3.1 Add a percent midpoint to `GradientColorStop`, a
  `GradientOpacityStop` list to `GradientSpec`, and spec (de)serialisation.
- [x] 3.2 Draw, hit-test, drag, add, delete, and select opacity stops above the
  ramp and midpoint diamonds inside it.
- [x] 3.3 Enable the Opacity / Location / Delete row for opacity stops and add
  a Midpoint field for either kind.
- [x] 3.4 Bake opacity stops and midpoints into the RGBA ramp used by Gradient
  Map, Gradient Fill, and the layer-style overlay.

## 4. Gradient tool

- [x] 4.1 Add the serialised gradient spec to `GradientOptions` and its JSON.
- [x] 4.2 Split the sample into an editor button and a preset arrow; store an
  accepted spec and clear it when a preset is chosen.
- [x] 4.3 Extend `draw_gradient` to accept the spec text and parse it into the
  engine model, falling back to the preset index.

## 5. Close out

- [x] 5.1 Update `ROADMAP.md`, `PARITY.md`, and `docs/dev/STATE.md`.
- [x] 5.2 Run fmt, clippy, the focused Rust and Qt suites, then the fast gate.
- [x] 5.3 Archive the OpenSpec change.
