# imaging/adaptive-wide-angle Specification

## Purpose
`Filter ▸ Adaptive Wide Angle…`: the lens model and base projection, the
constraint solver, the commit, and the full-canvas workspace.

## Requirements

### Requirement: Adaptive Wide Angle projection

The system SHALL implement `Filter::AdaptiveWideAngle` with a Correction of
Fisheye (equidistant source), Perspective (rectilinear source) or Full
Spherical (equirectangular source), a focal length in pixels of Focal Length ×
Crop Factor ÷ 43.27 × the layer diagonal, and Scale and an offset applied
about the layer centre. Fisheye and Perspective SHALL map through a
stereographic projection with unit magnification at the centre, Fisheye
clipped at 90° off-axis; Full Spherical with no constraints SHALL leave the
layer unchanged, and Full Spherical on a layer that is not 2:1 SHALL be
refused. Output pixels no source covers SHALL be transparent, or white
when the buffer has no alpha plane. The result SHALL be deterministic.

#### Scenario: A short fisheye collapses to a disc [awa_fisheye_disc]

- **WHEN** `adaptive_wide_angle::tests::short_fisheye_leaves_a_disc` applies
  Fisheye at 11.74 mm, crop 0.20, Scale 50 % to an opaque layer
- **THEN** the opaque area is a disc about the centre whose radius is the
  focal length in pixels within two pixels, and every pixel outside it is
  transparent

#### Scenario: Perspective pulls the corners in [awa_perspective_corners]

- **WHEN** `adaptive_wide_angle::tests::perspective_pulls_corners_in`
  applies Perspective at a short focal length and Scale 150 %
- **THEN** the corners turn transparent while the edge midpoints stay
  covered, and the centre pixel keeps its colour within 2

#### Scenario: Full Spherical without constraints is the identity [awa_spherical_identity]

- **WHEN** `adaptive_wide_angle::tests::full_spherical_is_identity_without_constraints`
  applies Full Spherical at Scale 100 % with no constraints to a 2:1 layer
- **THEN** every sample is unchanged

#### Scenario: Full Spherical is refused off 2:1 [awa_spherical_refused]

- **WHEN** `adaptive_wide_angle::tests::full_spherical_is_refused_off_two_to_one`
  applies Full Spherical to a 3:2 layer
- **THEN** the filter returns an error and the layer is unchanged

#### Scenario: Scale shrinks the result about the centre [awa_scale]

- **WHEN** `adaptive_wide_angle::tests::scale_shrinks_about_the_centre`
  applies Full Spherical at Scale 50 %
- **THEN** about a quarter of the layer stays opaque, centred

#### Scenario: A layer without transparency fills blanks white [awa_background_fill]

- **WHEN** `adaptive_wide_angle::tests::blank_areas_are_white_without_alpha`
  applies a correction that leaves blanks to a three-plane buffer
- **THEN** the blank pixels are white in every plane

#### Scenario: Repeated runs are identical [awa_deterministic]

- **WHEN** `adaptive_wide_angle::tests::constrained_runs_are_deterministic`
  applies the same constrained parameters twice
- **THEN** the outputs are byte-identical

### Requirement: Adaptive Wide Angle constraints

A line constraint SHALL trace the great circle between its two points under
the lens model, and the solve SHALL make that trace straight in the output.
A Horizontal constraint SHALL come out horizontal, a Vertical one vertical,
and a Fixed one at its angle. A polygon constraint SHALL straighten each edge.
A constraint that cannot be computed SHALL be reported invalid and SHALL NOT
change the output.

#### Scenario: A curved edge comes out straight [awa_line_straight]

- **WHEN** `adaptive_wide_angle::tests::a_line_constraint_straightens_its_trace`
  adds an Unfixed line across a fisheye layer off its centre
- **THEN** before the solve the trace bows more than eight pixels from its
  chord, and after it no sample is more than one pixel from the chord

#### Scenario: Horizontal and Vertical constraints level their trace [awa_line_orientation]

- **WHEN** `adaptive_wide_angle::tests::oriented_lines_level_their_trace`
  adds a slanted Horizontal line and, separately, a slanted Vertical line
- **THEN** the Horizontal trace's samples share one output row and the
  Vertical trace's one output column, within one pixel

#### Scenario: A fixed angle is kept [awa_fixed_angle]

- **WHEN** `adaptive_wide_angle::tests::a_fixed_line_takes_its_angle`
  fixes a line at 30°
- **THEN** its output endpoints lie at 30° within half a degree

#### Scenario: A polygon straightens its edges [awa_polygon]

- **WHEN** `adaptive_wide_angle::tests::a_polygon_straightens_its_edges`
  adds a four-corner polygon on a fisheye layer
- **THEN** each edge's trace lies within one pixel of its chord

#### Scenario: An invalid constraint is ignored [awa_invalid]

- **WHEN** `adaptive_wide_angle::tests::an_invalid_constraint_changes_nothing`
  adds a zero-length line
- **THEN** it is reported invalid and the output equals the output without it

### Requirement: Adaptive Wide Angle workspace

`Filter ▸ Adaptive Wide Angle…` SHALL be enabled when filters are and the
document is 8-bit RGB or Grayscale, and SHALL open a workspace over the active
layer titled `Adaptive Wide Angle (<document> @ <zoom>%)`. It SHALL offer the
Constraint (C), Polygon Constraint (Y), Move (M), Hand (H) and Zoom (Z)
tools; Correction (Fisheye, Perspective, Full Spherical; Full Spherical only
for a 2:1 layer), Scale (50–150 %, 100), Focal Length, Crop Factor
(0.10–10.00) and As Shot; Focal Length's range SHALL follow Crop Factor so
their product stays within 2.31–69.20 mm (4.05–121.41 mm at crop 0.57, which
the workspace opens at without EXIF); Preview (on), Show Constraints (on) and Show
Mesh (off); a Detail view; and Camera Model and Lens Model lines. EXIF Focal
Length SHALL prefill Focal Length and Crop Factor and enable As Shot, which is
otherwise disabled. Constraints SHALL be drawn cyan when unfixed, yellow when
horizontal, magenta when vertical, green when fixed, red when invalid. Edits
SHALL preview in the workspace without changing the document. Cancel SHALL
leave the document and history untouched; OK SHALL commit one filter history
state, and Last Filter Settings SHALL reopen the workspace with the committed
settings and constraints. Every path, Last Filter and Last Filter Settings
included, SHALL refuse a document that is not 8-bit RGB or Grayscale and Full
Spherical on a layer that is not 2:1; Last Filter Settings naming a
constraint set no longer kept SHALL be refused rather than open on defaults.

#### Scenario: The menu row follows the document depth [awa_menu]

- **WHEN** `tst_adaptive_wide_angle::menuFollowsDepth` opens an 8-bit RGB
  document, then converts a new one to 16 Bits/Channel
- **THEN** the command is enabled for the first and disabled for the second

#### Scenario: The workspace opens with CS6's controls [awa_controls]

- **WHEN** `tst_adaptive_wide_angle::opensWithControls` opens the workspace
  on an image with no EXIF
- **THEN** the title reads `Adaptive Wide Angle (<name> @ 100%)`, Correction
  lists Fisheye, Perspective and Full Spherical, Scale reads 100 within
  50–150,
  As Shot is disabled, Preview and Show Constraints are on, Show Mesh is off,
  and the camera line reads `--`; Crop Factor reads 0.57 within 0.10–10.00
  and Focal Length 75.91 within 4.05–121.41, and Crop Factor 0.10 re-ranges
  Focal Length to 23.1–692.04

#### Scenario: EXIF prefills the lens [awa_exif]

- **WHEN** `tst_adaptive_wide_angle::exifPrefillsLens` opens the workspace
  on a document whose EXIF gives a 50 mm focal length, Make and Model
- **THEN** Focal Length reads 50.00 and Crop Factor 1.00, As Shot is enabled
  and checked, and the camera line names the make and model

#### Scenario: Full Spherical needs a 2:1 layer [awa_spherical_gate]

- **WHEN** `tst_adaptive_wide_angle::fullSphericalNeedsTwoToOne` opens the
  workspace on a 3:2 layer, then on a 2:1 layer
- **THEN** Full Spherical is disabled for the first and enabled for the
  second

#### Scenario: A drag draws a constraint and Shift orients it [awa_draw_line]

- **WHEN** `tst_adaptive_wide_angle::dragDrawsConstraints` drags with the
  Constraint tool, then Shift-drags a mostly horizontal line
- **THEN** two constraints exist, the first unfixed and cyan, the second
  horizontal and yellow, and the preview changed

#### Scenario: The context menu sets orientation and deletes [awa_context_menu]

- **WHEN** `tst_adaptive_wide_angle::contextMenuOrientsAndDeletes` picks
  Vertical, then Arbitrary, then Delete from a constraint's menu
- **THEN** it turns magenta, then green, then is removed

#### Scenario: Polygon clicks close a polygon [awa_polygon_tool]

- **WHEN** `tst_adaptive_wide_angle::polygonClicksClose` clicks four
  corners with the Polygon Constraint tool, presses E once, clicks a new
  fourth corner and clicks the first again
- **THEN** one closed polygon constraint with four corners exists

#### Scenario: Preview off shows the uncorrected layer [awa_preview_toggle]

- **WHEN** `tst_adaptive_wide_angle::previewToggleShowsSource` turns
  Preview off with a correction that leaves blanks
- **THEN** the canvas shows the layer unchanged; turning it on shows the
  correction again

#### Scenario: Show Mesh draws the warped grid [awa_show_mesh]

- **WHEN** `tst_adaptive_wide_angle::showMeshDrawsGrid` turns Show Mesh on
- **THEN** the canvas draws a mesh path that follows the correction

#### Scenario: Move offsets the result [awa_move]

- **WHEN** `tst_adaptive_wide_angle::moveToolOffsets` drags 20 px with the
  Move tool at 100 %
- **THEN** the offset slots record 20 px of the layer size

#### Scenario: Tool shortcuts select tools [awa_tool_keys]

- **WHEN** `tst_adaptive_wide_angle::toolShortcuts` presses Y, M, H, Z
  and C
- **THEN** each selects its tool

#### Scenario: Cancel discards and OK commits one state [awa_commit]

- **WHEN** `tst_adaptive_wide_angle::cancelAndOk` draws a constraint and
  cancels, then draws one and presses OK
- **THEN** Cancel left the pixels and history unchanged; OK added one
  history state with the `adaptive-wide-angle` kind and changed the pixels

#### Scenario: Last Filter Settings restores the constraints [awa_last_settings]

- **WHEN** `tst_adaptive_wide_angle::lastSettingsRestoresConstraints`
  commits with Perspective, Scale 120 and one constraint, then reopens the
  dialog with the committed slots
- **THEN** it shows Perspective, 120 and one constraint

#### Scenario: The Detail view follows the pointer [awa_detail]

- **WHEN** `tst_adaptive_wide_angle::detailFollowsPointer` moves the
  pointer over two different parts of the preview
- **THEN** the Detail view's picture differs between them

#### Scenario: Last Filter refuses Full Spherical off 2:1 [awa_last_aspect]

- **WHEN** `tst_adaptive_wide_angle::lastFilterRefusesFullSphericalOffTwoToOne`
  commits Full Spherical on a 2:1 picture, makes the canvas 3:2, then runs
  Last Filter Settings and Last Filter
- **THEN** no workspace opens, no state is recorded, the pixels are
  unchanged, and the refusal names 2:1

#### Scenario: Last Filter refuses a document that is not 8-bit [awa_last_depth]

- **WHEN** `tst_adaptive_wide_angle::lastFilterRefusesOffEightBit` commits
  the filter, converts the document to 16 Bits/Channel, then runs Last Filter
  Settings and Last Filter
- **THEN** no workspace opens, no state is recorded, and the refusal names
  8-bit

#### Scenario: Last Filter Settings refuses a constraint set no longer kept [awa_unkept_set]

- **WHEN** `tst_adaptive_wide_angle::lastSettingsRefusesAnUnkeptSet` opens
  the workspace with slots naming a constraint set that is not kept
- **THEN** it is refused and no workspace opens
