# Proposal

## Why

Three Free Transform options-bar gaps:

- Clicking a reference-point locator cell moves the pivot but only the centre
  cell is ever drawn highlighted, so the active reference point is not visible.
- The X/Y fields call the pivot-only setter, so typing a document position moves
  the reference point inside the object and leaves the object where it was; the
  user expects X/Y to reposition the object.
- The W/H slider popups are linear over 0.1%-10000%, so the first pixels jump
  hundreds of percent. The useful 0%-100% band is a sliver of the slider.

## What Changes

- The locator highlights the cell holding the reference point (or no cell when
  the pivot is between anchors).
- X/Y set the reference point's document position and translate the target, so
  the object moves and the preview updates before Apply. Relative positioning
  treats the values as offsets from a fixed session origin.
- The W/H sliders are split at their midpoint: the first half spans the field
  minimum to 100%, the second half 100%-1000%, and 1000% is the typed maximum.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/transform-options-bar`: the locator highlight, X/Y translation, and the
  split W/H scale slider.

## Impact

- `crates/pictura-app/src/cxxqt_object/impl_transform/geometry.rs`: translate
  and pivot-preset helpers.
- `crates/pictura-app/src/cxxqt_object/impl_transform/bridge.rs` + `imp.rs`:
  `transform_set_position` and `transform_pivot_preset`.
- `crates/pictura-app/cpp/options_bar_transform.cpp` + `tool_transform.cpp`:
  the locator highlight and the X/Y wiring.
- `crates/pictura-app/cpp/panels/numeric_field.*`: the split slider mapping.
- Tests: `tst_transform_options` plus the geometry units.
- No new dependencies or document fields.
