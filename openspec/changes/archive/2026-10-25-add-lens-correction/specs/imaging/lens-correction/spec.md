# imaging/lens-correction Specification

## Purpose
`Filter ▸ Lens Correction…`: the correction model, edge fill, lens
profiles, the depth gate, the dialog, and the commit.

## ADDED Requirements

### Requirement: Lens Correction geometry

The system SHALL implement `Filter::LensCorrection` as one inverse map per
output pixel through Scale about the centre, an angle-and-perspective
homography, Remove Distortion, Auto Scale, and the profile's radial
distortion, resampled once. Positive Remove Distortion SHALL remove barrel
distortion; negative SHALL bulge the centre while the corners stay put, so
the frame stays filled. Positive Vertical Perspective SHALL widen the top,
positive Horizontal Perspective SHALL narrow the right side, and the
perspective sliders alone SHALL leave no blank area; a positive Angle SHALL
turn clockwise about the centre. All-default settings SHALL leave the
layer bit-identical, a non-positive Scale SHALL be refused, and the result
SHALL be deterministic.

#### Scenario: Default settings change nothing [lc_identity]

- **WHEN** `lens_correction::tests::default_params_are_bit_identical`
  applies the default settings (any Midpoint or Edge, Angle 360)
- **THEN** every sample is unchanged

#### Scenario: Remove Distortion keeps the centre and moves the edges [lc_distortion]

- **WHEN** `lens_correction::tests::distortion_keeps_centre_and_moves_edges_monotonically`
  walks a ray to the corner at Remove Distortion −100…+100
- **THEN** the centre maps to itself and the source radius grows along the
  ray; positive values sample nearer the centre toward the corner, more as
  the value rises; −100 magnifies the centre 2.5× and keeps the corner

#### Scenario: Vertical Perspective widens the top [lc_vertical]

- **WHEN** `lens_correction::tests::vertical_perspective_widens_the_top_and_mirrors`
  and `lens_correction::tests::vertical_perspective_converges_verticals` map
  points at Vertical +60 and −60, and +50
- **THEN** the centre is fixed, a row at the top shows a narrower span of the
  source than a row at the bottom, −60 is +60 flipped top to bottom, and an
  output vertical line maps to source points converging upward

#### Scenario: Horizontal Perspective narrows the right side [lc_horizontal]

- **WHEN** `lens_correction::tests::horizontal_perspective_narrows_the_right_side`
  maps the side columns at Horizontal +60
- **THEN** a column at the right shows a taller span of the source than one
  at the left

#### Scenario: Perspective alone leaves no blank [lc_perspective_fill]

- **WHEN** `lens_correction::tests::perspective_alone_leaves_no_blank` maps
  the frame corners at several Vertical and Horizontal values and applies
  Vertical +100 with Horizontal +70 to an opaque layer
- **THEN** every corner shows a point on the layer and every pixel stays
  opaque

#### Scenario: Angle turns clockwise about the centre [lc_angle]

- **WHEN** `lens_correction::tests::angle_rotates_clockwise_about_the_centre`
  maps a square layer at Angle 90
- **THEN** the centre is fixed and the source's top-left corner shows at the
  top right

#### Scenario: Scale zooms about the centre [lc_scale]

- **WHEN** `lens_correction::tests::scale_zooms_about_the_centre` maps a
  point at Scale 200 %
- **THEN** it shows the source point halfway to the centre

#### Scenario: A non-positive Scale is refused [lc_scale_refused]

- **WHEN** `lens_correction::tests::zero_scale_is_refused` applies Scale 0
- **THEN** the filter returns an error

#### Scenario: Straighten levels the drawn line [lc_straighten]

- **WHEN** `lens_correction::tests::straighten_levels_a_line` and the
  bridge's `straighten_returns_the_levelling_angle` measure a line leaning
  clockwise and set Angle to undo it
- **THEN** the deviation is the line's lean within (−45°, 45°] whichever way
  it was drawn, the Angle wraps into 0–360, and a level output row through
  the centre shows the leaning line

#### Scenario: Results are deterministic [lc_deterministic]

- **WHEN** `lens_correction::tests::deterministic` applies the same settings
  twice
- **THEN** the outputs are byte-identical

### Requirement: Lens Correction colour

Fix Red/Cyan, Green/Magenta, and Blue/Yellow Fringe SHALL each scale only
their channel radially about the centre, alpha following the green sample.
Vignette Amount SHALL darken (negative, to black at −100) or lighten
(positive, two stops at +100) toward the corners in linear light, leaving
the centre unchanged, and a lower Midpoint SHALL widen the area it affects.
A profile's vignette falloff SHALL be divided out.

#### Scenario: Each fringe slider scales only its channel [lc_fringe]

- **WHEN** `lens_correction::tests::fringe_scales_only_its_channel` applies
  each fringe slider alone
- **THEN** only that channel's plane changes (a flat plane stays as it is),
  red at +100 samples further from the centre and blue at −100 nearer

#### Scenario: Vignette darkens or lightens the corners [lc_vignette]

- **WHEN** `lens_correction::tests::vignette_darkens_and_lightens_corners`,
  `lens_correction::tests::geometry_free_settings_keep_every_sample_position`
  and `tst_lens_correction::vignetteLightensThePreview` apply Amount −100
  and +100 at several midpoints to flat grey and to the dialog's preview
- **THEN** the centre keeps its value, the corners darken or lighten,
  Midpoint 0 darkens a mid-radius point that Midpoint 100 leaves alone, and
  a mid-tone near the preview's edge visibly lifts and darkens

### Requirement: Lens Correction edge fill

Blank areas SHALL follow Edge: Transparency clears them (white on a buffer
with no alpha plane, while the preview shows them clear), Black Color and
White Color fill them opaque, and Edge Extension repeats the nearest edge
pixel. Pixels partly on the layer SHALL blend with the fill by coverage.
With Auto Scale Image on, a profile's distortion SHALL leave no blank area;
Auto Scale SHALL NOT compensate the Custom transform. The preview SHALL
render the same pixels the commit writes.

#### Scenario: Transparency clears blanks [lc_edge_transparent]

- **WHEN** `lens_correction::tests::edge_transparency_clears_or_whitens`
  shrinks a layer to 50 %
- **THEN** the corner is clear with an alpha plane, white without one, and
  the centre stays opaque

#### Scenario: Black and White fill opaque [lc_edge_color]

- **WHEN** `lens_correction::tests::edge_colours_fill_opaque` shrinks a
  layer with Black Color and White Color
- **THEN** the corners are opaque black or white

#### Scenario: Edge Extension repeats the edge [lc_edge_extend]

- **WHEN** `lens_correction::tests::edge_extension_repeats_the_edge` shrinks
  a layer with Edge Extension
- **THEN** the corners repeat the source's corner pixels

#### Scenario: Auto Scale leaves no blank [lc_auto_scale]

- **WHEN** `lens_correction::tests::auto_scale_leaves_no_blank` applies a
  pincushion profile with Auto Scale off, on, and on with Angle 30
- **THEN** off leaves clear corners, on leaves every pixel opaque, and the
  rotation still leaves clear corners

#### Scenario: The preview matches the commit [lc_preview_match]

- **WHEN** `lens_correction::tests::preview_render_matches_commit` renders
  the preview at 100 % and commits the same settings
- **THEN** alpha matches exactly and opaque colour within one level

### Requirement: Lens Correction profiles

Lens profiles SHALL be read from an in-tree text format whose parse errors
name the line. Matching SHALL need the camera make (case-insensitive) and
rank a matching model and lens higher; an empty criterion matches anything.
The Auto Correction checkboxes SHALL gate the profile's distortion,
chromatic aberration, and vignette parts, and the profile SHALL compose with
the Custom settings. Slots naming a profile that is not installed SHALL be
refused, and slot values SHALL be clamped to their ranges.

#### Scenario: Profiles parse and report errors by line [lc_profile_parse]

- **WHEN** `lens_correction::tests::profile_parse_reports_the_line` and
  `lens_correction::tests::builtin_profiles_parse` parse good and bad text
- **THEN** the bundled set parses, and a short triple, a key outside a
  block, an unknown key, and a profile with no make are errors naming the
  line or profile

#### Scenario: Profiles match by make, model, and lens [lc_profile_match]

- **WHEN** `lens_correction::tests::profiles_match_make_then_model_and_lens`
  searches three profiles
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
  non-finite value or a wrong count is refused, and a profile id past the
  bundled set is refused

### Requirement: Lens Correction depth gate and menu

`Filter ▸ Lens Correction…` SHALL be enabled only for 8- or 16-bit RGB and
Grayscale documents, carry the `Shift+Ctrl+R` shortcut, and every commit
path (OK, Last Filter, Last Filter Settings) SHALL refuse other documents
with a message naming the requirement.

#### Scenario: The menu follows depth and mode [lc_menu_gate]

- **WHEN** `tst_lens_correction::menuFollowsDepthAndMode` and the bridge's
  `commits_are_refused_off_eight_bit_rgb_and_gray` check 8-bit RGB, 16-bit,
  32-bit, Grayscale, CMYK, and Lab documents
- **THEN** the row is enabled for 8- and 16-bit RGB and Grayscale only, and
  Adaptive Wide Angle and Vanishing Point stay 8-bit only

#### Scenario: Shift+Ctrl+R opens it [lc_shortcut]

- **WHEN** `tst_lens_correction::menuRowHasItsShortcut` reads the action
- **THEN** its shortcut is `Shift+Ctrl+R` and its text "Lens Correction…", one
  ellipsis (Liquify's too)

#### Scenario: Last Filter refuses off the gate [lc_last_gate]

- **WHEN** `tst_lens_correction::lastFilterRefusesOffSixteenBit` commits,
  converts to 16 bits and runs Last Filter, then converts to 32 bits and runs
  Last Filter Settings and Last Filter
- **THEN** 16-bit applies; at 32 bits no dialog opens, no state is added,
  and the refusal names "8- or 16-bit"

### Requirement: Lens Correction dialog

The dialog SHALL offer CS6's tools (Remove Distortion D, Straighten A, Move
Grid M, Hand H, Zoom Z) under a help line naming the active tool; the Auto
Correction tab (Geometric Distortion, Chromatic Aberration, Vignette, Auto
Scale Image, Edge, Camera Make / Model / Lens Model criteria filled from
EXIF, the Lens Profiles list showing "No matching profiles found" with no
match, a disabled Search Online); the Custom tab (Settings, Remove
Distortion, three fringe sliders with two decimals, Vignette Amount and
Midpoint, Vertical and Horizontal Perspective, Angle with a dial, Scale
50–150 %); and Preview, Show Grid, grid Color and Size (64) with the Camera
Model, Lens Model, and Camera Settings lines.

#### Scenario: The dialog opens with CS6's controls [lc_controls]

- **WHEN** `tst_lens_correction::opensWithControls` opens the dialog on an
  image without EXIF
- **THEN** the title is "Lens Correction (100%)", the tabs, Edge items,
  ranges, and defaults are CS6's, the profile list says "No matching
  profiles found", Search Online is disabled, and the status lines show `--`

#### Scenario: EXIF fills the criteria and status [lc_exif]

- **WHEN** `tst_lens_correction::exifPrefillsSearchAndStatus` and the
  bridge's `camera_settings_text` open on `metadata.psd`
- **THEN** Camera Make and Model are filled, the status names the camera and
  "50mm, f/2.8", the matching profile is chosen and its checkboxes apply,
  and clearing the make leaves no profile

#### Scenario: Settings switches to Custom on an edit [lc_settings]

- **WHEN** `tst_lens_correction::settingsComboSwitchesToCustom` edits a
  slider and then picks Default Correction
- **THEN** Settings reads Custom after the edit, and Default Correction
  resets Remove Distortion but leaves the perspective

#### Scenario: Remove Distortion drags the slider [lc_tool_distort]

- **WHEN** `tst_lens_correction::removeDistortionToolDragsSlider` drags
  toward the centre, then away
- **THEN** Remove Distortion turns positive, then negative

#### Scenario: Straighten sets the Angle [lc_tool_straighten]

- **WHEN** `tst_lens_correction::straightenToolSetsAngle` drags a line
  leaning clockwise
- **THEN** Angle becomes 360° minus the lean, and Settings reads Custom

#### Scenario: The Angle dial and field agree [lc_angle_dial]

- **WHEN** `tst_lens_correction::angleDialAndFieldSync` sets the field and
  turns the dial
- **THEN** the dial points up at 0° and turns clockwise with the field, and
  turning the dial sets the field

#### Scenario: Move Grid moves the grid [lc_tool_grid]

- **WHEN** `tst_lens_correction::moveGridToolOffsetsGrid` drags with Move
  Grid
- **THEN** the grid's offset follows the drag and wraps at a cell

#### Scenario: Show Grid, Size, and Color [lc_grid]

- **WHEN** `tst_lens_correction::showGridSizeAndColor` shows the grid at
  size 32
- **THEN** the grid is drawn every 32 px across the layer in the swatch's
  colour

#### Scenario: Tool keys select the tools [lc_tool_keys]

- **WHEN** `tst_lens_correction::toolShortcuts` presses A, M, H, Z, D
- **THEN** each selects its tool on the dialog and the canvas

#### Scenario: The help line follows the tool [lc_hint]

- **WHEN** `tst_lens_correction::hintFollowsTool` selects each tool
- **THEN** the help line reads that tool's hint

#### Scenario: Preview off shows the layer as it is [lc_preview]

- **WHEN** `tst_lens_correction::previewToggleShowsSource` shrinks to 50 %,
  turns Preview off and on, then picks Black Color
- **THEN** the blank corner is clear, then the layer's own pixel, then
  clear again, then black

#### Scenario: Grayscale has no fringe to fix [lc_gray]

- **WHEN** `tst_lens_correction::grayscaleDisablesChromaticAberration`
  opens on a Grayscale document
- **THEN** the fringe sliders and the Chromatic Aberration checkbox are
  disabled and its slot is 0

### Requirement: Lens Correction commit

Cancel SHALL leave the document unchanged; OK SHALL add one history state
through the ordinary filter path and record the kind's 16 slots for Last
Filter and Last Filter Settings. Previous Conversion SHALL restore the last
OK's distortion, fringe, and vignette values, never the transform.

#### Scenario: Cancel changes nothing; OK commits one state [lc_commit]

- **WHEN** `tst_lens_correction::cancelAndOk` cancels and then accepts a
  Remove Distortion of 40
- **THEN** Cancel leaves the history and pixels; OK adds one state, records
  `lens-correction` with 16 slots, and changes the pixels

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

### Requirement: Lens Correction camera and lens catalogues

The Auto Correction tab's Search Criteria SHALL list camera makes and
models from a bundled camera catalogue (crop factor, sensor, and a fixed
lens's range and aperture per body) and lens models from a bundled lens
catalogue (crop factor and which corrections lensfun calibrates per lens),
with the photo's EXIF names mapped to the catalogue's spelling. Lens models
SHALL be listed with the camera's own make first, each group nearest the
camera's crop factor first, and, when EXIF gives a focal length, only lenses
whose range covers it. A line SHALL describe the chosen camera and lens. The
catalogues SHALL carry no correction data: choosing a catalogue lens SHALL
NOT select a profile.

#### Scenario: The catalogues load [lc_catalog]

- **WHEN** `lens_correction::tests::catalogues_load` reads both catalogues
- **THEN** there are over 3,000 bodies and 1,500 lenses, a full-frame body
  has crop 1 and no fixed lens, a compact keeps its fixed lens's range and
  aperture, and a lens keeps its calibrated flags and focal range

#### Scenario: Lens names give their range and aperture [lc_lens_names]

- **WHEN** `lens_correction::tests::lens_names_give_their_range_and_aperture`
  parses prime, zoom, variable-aperture, and nameless-range lens names
- **THEN** each gives its focal range and maximum aperture, or none

#### Scenario: EXIF names find the catalogue spelling [lc_exif_names]

- **WHEN** `lens_correction::tests::exif_names_find_the_catalogue_spelling`
  and the bridge's `exif_names_take_the_catalogue_spelling` look up EXIF
  makes, models, and lenses such as "NIKON CORPORATION" / "NIKON D850"
- **THEN** they resolve to the catalogue's "Nikon" / "D850", and unknown
  names pass through unchanged

#### Scenario: Lenses follow the camera and its settings [lc_lens_settings]

- **WHEN** `lens_correction::tests::lenses_follow_the_camera_and_its_settings`
  and `tst_lens_correction::exifPrefillsSearchAndStatus` list lenses for a
  camera with and without a focal length
- **THEN** the camera's make comes first, the nearest crop factor first
  within it, and a focal length keeps only lenses covering it

#### Scenario: The criteria describe the camera and lens [lc_criteria]

- **WHEN** `tst_lens_correction::criteriaListCamerasAndLenses` and the
  bridge's `criteria_info_describes_camera_and_lens` pick Canon, EOS 5D
  Mark III, and a 24-70 mm lens
- **THEN** the makes include the major brands, the models and lenses fill
  from the catalogues, the line gives the crop factor, sensor, lens range,
  aperture, and calibrated corrections, and no profile is selected
