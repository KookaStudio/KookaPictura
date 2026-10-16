## ADDED Requirements

### Requirement: Adjustment parameter descriptors and edits

The engine SHALL describe each supported adjustment's controls (key, label,
range or kind, group, value, default) and SHALL set one parameter by patching
only the data that holds it, clamped to its range, refusing a result that does
not decode, so that data the model does not carry is preserved.

#### Scenario: Edits keep unmodelled data and refuse invalid results

- **WHEN** the `adjustment_params` unit tests edit Brightness/Contrast, Hue/Saturation (with Colorize and a range record set), Levels (with per-channel records), Vibrance (a descriptor with an extra item), Color Balance, Selective Color, Channel Mixer, Photo Filter v2 / v3, and Curves, clamp out-of-range values, cross Levels' input black past its white, and reset
- **THEN** each edit reads back, the unmodelled bytes and items survive, values clamp, the invalid Levels edit is refused, Photo Filter v3 offers no colour, Curves edits per channel, and Reset restores the defaults

### Requirement: Properties panel editing

The Properties panel SHALL build an adjustment layer's controls from its
descriptor, apply edits to the canvas live, and record one "Modify `Title`
Layer" state per gesture; it SHALL offer Clip to Layer, Reset, Toggle
Visibility, and Delete, show a read-only summary for any other layer, and read
No Properties without a layer.

#### Scenario: Live edits, one state, groups, and the footer

- **WHEN** the `tst_properties_panel` test drags a Hue/Saturation slider, commits, and undoes; switches Color Balance's tone menu, edits, toggles Preserve Luminosity, resets, hides, and deletes; edits Threshold; and selects a plain layer and no document
- **THEN** the canvas changes before any state is recorded, one "Modify Hue/Saturation Layer" state follows and undo restores the slider and pixels, only the chosen tone's sliders show, Reset restores the defaults, the layer hides and is deleted, Threshold commits under its name, the plain layer shows its kind / size / blend / opacity, and no document reads No Properties
