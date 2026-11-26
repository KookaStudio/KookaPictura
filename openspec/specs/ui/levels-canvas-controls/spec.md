# Levels canvas controls

## Purpose

Define canvas point sampling and clipping feedback in the Levels dialog.

## Requirements

### Requirement: Levels can pick tonal points from the canvas

Image > Adjustments > Levels SHALL provide black, gray, and white point eyedroppers for RGB pixel targets. The black and white pickers SHALL also support Grayscale. The eyedroppers SHALL expose neutral target values and a right-click sample-size menu with Point (default), 3×3, 5×5, 11×11, 31×31, 51×51, and 101×101 choices, use the composite before the current Levels preview, and stage an edit in the normal dialog transaction. Gray targets SHALL be integers from 1 through 254, with gamma bounded by the existing Levels limits.

#### Scenario: Pick an RGB endpoint with a custom target [lev_pick_endpoint]

- **GIVEN** an RGB image with an opaque sample whose components are in 0–253 for black or 2–255 for white
- **WHEN** the user sets a neutral target and picks the sample with the black or white eyedropper
- **THEN** the dialog SHALL replace prior Levels edits with per-component input endpoints and corresponding output floors or ceilings that send the picked sample to the target within normal Levels rounding
- **AND** Cancel SHALL leave original pixels intact

#### Scenario: Neutralize a gray point [lev_pick_gray]

- **GIVEN** an RGB sample whose components are within the workable gamma range
- **WHEN** the user picks it with the default gray eyedropper
- **THEN** its three channels SHALL map to approximately mid-gray
- **AND** accepting the dialog SHALL commit one Levels edit

#### Scenario: Sample sizes and unsupported picks [lev_pick_limits]

- **WHEN** the user selects an averaged sample size and picks an opaque neighborhood
- **THEN** the sampled point SHALL be the neighborhood average
- **AND** a fully transparent sample SHALL leave the block unchanged
- **AND** a black or white pick with any affected component outside its supported input range SHALL leave the block unchanged
- **AND** a gray pick containing a component at an endpoint SHALL leave the block unchanged
- **AND** a rejected pick SHALL restore the regular Preview setting, including a pending or displayed preview when Preview is on
- **AND** the gray picker SHALL be unavailable in Grayscale

### Requirement: Alt-drag shows input clipping in Levels

Image > Adjustments > Levels SHALL show a temporary clipping map during an Alt-drag of the input black or white thumb for RGB and Grayscale pixel targets. The map SHALL use the current input threshold against the unadjusted composite. Its lifetime SHALL follow the gesture without adding a history state.

#### Scenario: Preview shadow and highlight clipping [lev_clip_drag]

- **GIVEN** a patch with RGB components on either side of the selected threshold
- **WHEN** the user Alt-drags the input black or input white thumb
- **THEN** the map SHALL show clipped components on white for shadows and on black for highlights
- **AND** releasing Alt or ending the drag SHALL remove the map
- **AND** the dialog SHALL keep the selected input value and resume the user's regular Preview setting

#### Scenario: Other gestures keep their usual preview [lev_clip_other]

- **WHEN** the user drags the gamma or output thumbs with Alt, or an input endpoint without Alt
- **THEN** the clipping map SHALL stay hidden
