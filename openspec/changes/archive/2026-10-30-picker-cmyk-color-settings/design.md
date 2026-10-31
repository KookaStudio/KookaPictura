# Design

`PickerTransforms` is the cached sRGB ↔ working-CMYK pair (#1037). It is built by
`picker_transforms`, which already took the profile; it now also takes the
`ConversionOptions` rendering intent and black-point flag. The options are held
beside the pair in a process-global, so `picker_options()` reports what the pair
was built with and the service tests can mirror the one-shot conversion.

`install_picker(icc, options)` writes the options and the rebuilt pair together.
`install_picker_profile(icc)` keeps the current options and is the profile-only
entry the tests use. `set_color_settings` already receives the intent,
black-point flag, and dither; it stores them on the view and now calls
`install_picker` instead of installing the profile alone. A changed option
therefore rebuilds the pair, which is the "PickerTransforms also has to be
rebuilt when they change" the issue asks for.

Dither is forced off for the picker: the pair is a `SpaceTransform`, which has no
dither stage, and the ordered pattern is an 8×8 spatial grid, meaningless for a
single colour. `picker_options()` keeps `dither: false` and the specs now say so.

Both the picker dialog and the Info panel read the pair through `rgbToCmyk`, so
they move together; that is the shared-conversion requirement, and it is why the
Info panel's CMYK readout changes with the same option.

The service tests share one installed pair and build counter under `cargo test`,
so `take_turn()` now resets the pair and its options to the defaults, keeping
each test independent.
