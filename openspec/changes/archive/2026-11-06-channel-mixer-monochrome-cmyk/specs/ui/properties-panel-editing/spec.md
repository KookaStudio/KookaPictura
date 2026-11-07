## MODIFIED Requirements

### Requirement: Adjustment parameter descriptors and edits

The engine SHALL describe each supported adjustment's controls (key, label,
range or kind, group, value, default) and SHALL set one parameter by patching
only the data that holds it, clamped to its range, refusing a result that does
not decode, so that data the model does not carry is preserved. The descriptor
and the edit SHALL take the document mode. For an RGB Channel Mixer the page
SHALL carry a `monochrome` check; for a CMYK Channel Mixer it SHALL carry the
`Cyan`, `Magenta`, `Yellow`, and `Black` groups, the source keys
`{output}.cyan`/`.magenta`/`.yellow`/`.black`, and no `monochrome` check.
Setting `monochrome` on an RGB mixer SHALL re-encode the block: enabling from the
identity rows seeds the Red row to `[40, 40, 20]`, otherwise it keeps the
current Red row; disabling keeps the Red row as the Gray mix. In both directions
the Green and Blue rows return to their identity values.

#### Scenario: Edits keep unmodelled data and refuse invalid results

- **WHEN** the `adjustment_params` unit tests edit Brightness/Contrast, Hue/Saturation (with Colorize and a range record set), Levels (with per-channel records), Vibrance (a descriptor with an extra item), Color Balance, Selective Color, Channel Mixer, Photo Filter v2 / v3, and Curves, clamp out-of-range values, cross Levels' input black past its white, and reset
- **THEN** each edit reads back, the unmodelled bytes and items survive, values clamp, the invalid Levels edit is refused, Photo Filter v3 offers no colour, Curves edits per channel, and Reset restores the defaults

#### Scenario: Monochrome toggles the Gray output [ppe_cm_monochrome_toggle]

- **WHEN** `monochrome` is set true on an RGB identity mixer and then false
- **THEN** the page's groups become `Gray` and the Red row `[40, 40, 20]`, and
  then return to `Red`/`Green`/`Blue` with that Red row and identity Green/Blue rows

#### Scenario: CMYK pages expose four ink outputs [ppe_cm_cmyk_groups]

- **WHEN** a Channel Mixer page is built for a CMYK document
- **THEN** its groups are `Cyan`, `Magenta`, `Yellow`, and `Black`, its source
  keys are the four ink sources per output, and every range comes from the layout

#### Scenario: CMYK pages have no Monochrome check [ppe_cm_monochrome_hidden_cmyk]

- **WHEN** a Channel Mixer page is built for a CMYK document
- **THEN** it carries no `monochrome` parameter
