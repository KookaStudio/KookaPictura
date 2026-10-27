# Spec Delta

## MODIFIED Requirements

### Requirement: Lens Correction profiles

Lens profiles SHALL be read from an in-tree text format whose parse errors
name the line. Matching a camera-specific profile SHALL need the camera make
(case-insensitive) and rank a matching model and lens higher; an empty
criterion matches anything. The Auto Correction checkboxes SHALL gate the
profile's distortion, chromatic aberration, and vignette parts, and the
profile SHALL compose with the Custom settings. Slots that carry no profile
coefficients and name a profile other than the bundled one SHALL be refused,
and slot values SHALL be clamped to their ranges.

#### Scenario: Profiles parse and report errors by line [lc_profile_parse]

- **WHEN** `lens_correction::tests::profile_parse_reports_the_line` and
  `lens_correction::tests::builtin_profiles_parse` parse good and bad text
- **THEN** the bundled set parses, and a short triple, a key outside a
  block, an unknown key, and a profile with no make are errors naming the
  line or profile

#### Scenario: Profiles match by make, model, and lens [lc_profile_match]

- **WHEN** `lens_correction::tests::profiles_match_make_then_model_and_lens`
  searches three camera-specific profiles
- **THEN** results need the make, a lens match outranks a model match, and
  no make matches nothing

#### Scenario: The profile composes with Custom [lc_profile_compose]

- **WHEN** `lens_correction::tests::profile_composes_with_custom` and the
  bridge's `checkboxes_gate_the_profile_parts` apply a vignette-only profile
  with and without a Custom darkening, and clear each checkbox
- **THEN** the profile brightens the corners, the Custom darkening pulls
  them back, and each cleared checkbox zeroes its part

#### Scenario: Slots map, clamp, and refuse an unknown profile [lc_slots]

- **WHEN** the bridge's `default_slots_are_the_identity`,
  `slots_map_and_clamp`, and `unknown_profile_id_is_refused` map slot lists
- **THEN** the defaults are the identity, out-of-range values clamp, a
  non-finite value or a wrong count is refused, and a profile id that is not
  the bundled one is refused when the slots carry no coefficients

### Requirement: Lens Correction commit

Cancel SHALL leave the document unchanged; OK SHALL add one history state
through the ordinary filter path and record the kind's 36 slots for Last
Filter and Last Filter Settings. Previous Conversion SHALL restore the last
OK's distortion, fringe, and vignette values, never the transform.

#### Scenario: Cancel changes nothing; OK commits one state [lc_commit]

- **WHEN** `tst_lens_correction::cancelAndOk` cancels and then accepts a
  Remove Distortion of 40
- **THEN** Cancel leaves the history and pixels; OK adds one state, records
  `lens-correction` with 36 slots, and changes the pixels

#### Scenario: Previous Conversion restores the last OK [lc_previous]

- **WHEN** `tst_lens_correction::previousConversionRestoresLastCommit`
  commits distortion, fringe, and perspective, then picks Previous
  Conversion in a new dialog
- **THEN** the item is enabled, distortion and fringe come back, and the
  perspective stays at 0

#### Scenario: Last Filter Settings restores the slots [lc_last_settings]

- **WHEN** `tst_lens_correction::lastSettingsRestoresValues` commits and
  runs Last Filter Settings
- **THEN** the dialog opens with the committed values and Cancel adds no
  state
