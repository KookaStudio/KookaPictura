# Design

`PickerTransforms` is already the installed working-profile pair, rebuilt by
`set_color_settings` and cached for drags (#1037). `picker_cmyk` already projects
through it, so `rgbToCmyk` calls that service and keeps its `CmykPercent` shape.
Both the picker and the Info panel use `rgbToCmyk`, so they stay in agreement.

`CmykConverter` is removed rather than left unused: its fixed coated profile is
what caused this bug. Its white, black, and round-trip tests move onto the picker
services.

The repo has one CMYK ICC, so the Rust test builds a second by repointing the
bundled profile's `B2A1` tag at its different `B2A0` table. Qt tests cannot install
a profile through `set_color_settings`, which maps every choice to coated, so they
pin the wiring: fields and `rgbToCmyk` equal `picker_cmyk`.

The service tests share one installed pair under `cargo test`, so the tests that
depend on its values take turns.
