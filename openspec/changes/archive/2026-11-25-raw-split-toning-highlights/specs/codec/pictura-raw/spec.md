## MODIFIED Requirements

### Requirement: Pictura Raw split toning

The Split Toning tab SHALL tint shadows and highlights with the selected hue
and saturation while preserving each pixel's luminance. A non-zero saturation on
one side SHALL tint that side even when the other side's saturation is zero. A
positive Balance SHALL bias the tint toward the highlights and a negative
Balance toward the shadows. Each Saturation slider's track SHALL run from
neutral grey to the colour of its paired Hue slider, and the Hue tracks SHALL
show a full hue wheel.

#### Scenario: Positive balance favours highlights [rs_balance]

- **WHEN** a mid-grey pixel renders with a red shadow tint, a blue highlight
  tint, and Balance +100, then with Balance -100
- **THEN** the positive balance leaves the pixel blue and the negative balance
  leaves it red

#### Scenario: Split toning preserves luminance [rs_luminance]

- **WHEN** a dark and a bright grey pixel render with opposite shadow and
  highlight tints
- **THEN** each pixel's luminance is unchanged within rounding

#### Scenario: Saturation tracks follow the hue [rs_saturation_ramp]

- **WHEN** the Highlights Hue is 167 and the Shadows Hue is 291
- **THEN** the matching Saturation tracks end in those hue colours, and the Hue
  tracks span the full wheel

#### Scenario: Highlights tint alone [rs_highlights_only]

- **WHEN** the Highlights Hue is 167 and Highlights Saturation is 85 while the
  Shadows Saturation is zero
- **THEN** a bright neutral pixel turns green-cyan and a dark neutral pixel is
  unchanged, and the dialog's preview follows the highlight sliders
