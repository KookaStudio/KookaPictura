# Proposal: paintbrush-preset-defaults

## Why

Issue #731 follow-up: the four brushes added for the issue need the colours
and paint settings in the owner's screenshots. The previous change applied the
tip only, so selecting a preset left the foreground, Mode, Opacity, and Flow at
whatever the user last used. The Non-Photo Blue Pencil therefore did not paint
blue or show its paper texture, and the Paintbrush Oval did not multiply at
35 % opacity and 50 % flow. The airbrush also painted a heavy, nearly solid
core: its 10 % spacing let the 50 % flow build up, while the owner's Photoshop
reference is airy and light. The 2B Pencil's binary grain dropped 75 % of its
pixels, scattering the stroke into isolated dots instead of texturing a
continuous line.

## What Changes

- `crates/pictura-app/cpp/panels/brush_preset_picker.cpp`: the four added
  presets carry a default foreground colour, Mode, Opacity, and Flow;
  `applyPreset` writes them. The airbrush uses 25 % spacing so its 50 % flow
  stays light; the pencils' grain is 30. The existing CS6 presets keep the
  tip-only behaviour.
- `crates/pictura-paint/src/stroke.rs`: grain modulates the accumulated alpha
  instead of dropping pixels to zero, so the pencils paint a continuous
  textured line.
- `crates/pictura-app/cpp/options_bar.cpp`, `tool_options.h`, `tools.cpp`:
  Opacity, Flow, and Mode changes notify the bar, so a preset cannot leave
  stale values in it.
- `crates/pictura-app/cpp/tests/tst_brush_tip_preview.cpp` and
  `crates/pictura-paint/src/stroke/tests.rs`: cover the new defaults, the
  options-bar sync, the airy airbrush profile, and the continuous pencil
  grain.
- `docs/dev/STATE.md`: record the defaults.

## Capabilities

**Modified:** `ui/brush-preset-picker`

## Impact

- Selecting one of the four added presets now changes the foreground colour
  and the paint settings; selecting an existing CS6 preset does not. The
  airbrush's core stays light, and the options bar shows the applied
  Opacity/Flow/Mode.
- No new dependency. The preview and the stroke engine are unchanged.
- Ceiling: the colours and settings come from the owner's screenshots, and the
  brush tips remain non-parity approximations.
