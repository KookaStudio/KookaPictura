# Spec Delta

## MODIFIED Requirements

### Requirement: Rectangular and elliptical marquee
The system SHALL create a selection from a dragged rectangle (Rectangular
Marquee) or ellipse (Elliptical Marquee) whose bounds are the drag rectangle. At
least one pixel covered at the pixel centre SHALL be selected. Both tools SHALL
be enabled in the toolbox, SHALL support New, Add, Subtract, and Intersect
combine modes, and SHALL apply a tool-time Feather radius (0-250 px, decimal) to
the rasterised shape before combining. The Style option SHALL constrain the drag
geometry before rasterisation: Normal follows the drag, Fixed Ratio keeps the
entered width-to-height ratio while staying inside the non-degenerate box the
cursor draws, and Fixed Size places the entered pixel size with its top-left
corner at the current pointer and follows the pointer until release. A one-axis
Fixed Ratio drag SHALL still derive the other axis from the entered ratio and MAY
extend outside the degenerate box. Fixed Ratio and Fixed Size SHALL round half
away from zero. A Fixed Size selection partly or wholly outside the canvas SHALL
be clipped by the rasteriser. Fixed Ratio and Fixed Size SHALL ignore Shift and
Alt. Anti-alias SHALL be offered for the Elliptical Marquee and SHALL be
functional when enabled: boundary pixels MAY receive fractional coverage
strictly between 0 and 255. When Anti-alias is disabled, every coverage value
SHALL be 0 or 255. The Rectangular Marquee SHALL NOT expose Anti-alias.
Fixed-ratio and fixed-size defaults, and non-pixel size units, are inferred and
MAY be deferred.

#### Scenario: Drag a rectangular selection
- **WHEN** the Rectangular Marquee tool is dragged from one point to another
- **THEN** the selection covers the dragged rectangle

#### Scenario: Drag an elliptical selection
- **WHEN** the Elliptical Marquee tool is dragged from one point to another
- **THEN** the selection covers the ellipse inscribed in the dragged rectangle

#### Scenario: Tool-time feather softens the boundary
- **WHEN** the Elliptical Marquee is dragged with Feather greater than zero
- **THEN** the committed selection has boundary pixels strictly between 0 and 255

#### Scenario: Fixed ratio constrains the geometry
- **WHEN** Style is Fixed Ratio with a 2:1 ratio and the tool is dragged
- **THEN** the committed selection's width-to-height ratio is 2 within 1 px

#### Scenario: Fixed ratio fits the drag box [lst_fixed_ratio_fits_drag]
- **WHEN** Style is Fixed Ratio with a 2:1 ratio and the cursor draws a 40x10 box
- **THEN** the selection is 20x10 inside that box, not 40x20 past the cursor

#### Scenario: Fixed ratio fits the narrow axis [lst_fixed_ratio_narrow]
- **WHEN** Style is Fixed Ratio with a 2:1 ratio and the cursor draws a 10x40 box
- **THEN** the selection is 10x5 inside that box

#### Scenario: A one-axis Fixed Ratio drag derives the other axis [lst_fixed_ratio_one_axis]
- **WHEN** Style is Fixed Ratio with a 2:1 ratio and the cursor draws a 0x40 box
- **THEN** the selection is 80x40 and may extend outside the degenerate box

#### Scenario: Fixed ratio fits a reverse drag [lst_fixed_ratio_reverse]
- **WHEN** Style is Fixed Ratio with a 2:1 ratio and the cursor drags from (50,50) to (10,10)
- **THEN** the selection is (10,30,40,20)

#### Scenario: Fixed size ignores the drag extent
- **WHEN** Style is Fixed Size with a 100x50 size and the pointer is dragged from the press point
- **THEN** only the size is fixed; the committed selection is 100x50 with its top-left corner at the release point

#### Scenario: Fixed size follows the pointer on a large document [lst_fixed_size_follows]
- **WHEN** Style is Fixed Size with a 100x50 size on a document large enough to hold it and the pointer is dragged to (120,80)
- **THEN** the selection is 100x50 with its top-left corner at (120,80)

#### Scenario: Fixed size clips at the canvas edge [lst_fixed_size_clip]
- **WHEN** Style is Fixed Size with a 100x50 size on a 64x64 document and the release point is (20,20)
- **THEN** the committed selection is clipped to the canvas

#### Scenario: The ellipse uses its own Fixed Ratio [lst_elliptical_fixed_ratio]
- **WHEN** the Elliptical Marquee is set to Fixed Ratio with a 2:1 ratio and the cursor draws a 40x10 box
- **THEN** the committed ellipse is inscribed in a 20x10 rectangle

#### Scenario: The ellipse uses its own Fixed Size [lst_elliptical_fixed_size]
- **WHEN** the Elliptical Marquee is set to Fixed Size with a 100x50 size on a document large enough to hold it and the pointer is dragged to (120,80)
- **THEN** the committed ellipse is inscribed in a 100x50 rectangle with its top-left corner at (120,80)

#### Scenario: Anti-alias softens the elliptical boundary [lst_aa_ellipse]
- **WHEN** the Elliptical Marquee is dragged with Anti-alias on and then again with it off
- **THEN** the mask committed with Anti-alias on contains boundary values strictly between 0 and 255, while the mask with it off contains only 0 and 255

#### Scenario: Anti-alias is honest about the ceiling
- **WHEN** the Elliptical Marquee options are inspected
- **THEN** the Anti-alias control is enabled, and the engine's inferred boundary sampling is treated as behavioural parity rather than Adobe pixel parity

### Requirement: Selection tool options bars
The system SHALL show context-sensitive options for each enabled selection tool:
combine mode (New/Add/Subtract/Intersect) and Feather for the marquee and lasso
tools, Tolerance and Contiguous for the Magic Wand, and mode plus Tolerance for
Quick Selection. Anti-alias SHALL be enabled and functional for the Elliptical
Marquee, the Lasso, the Polygonal Lasso, the Magnetic Lasso, and the Magic Wand.
Options that the engine does not model SHALL be visible and disabled with a
stated reason: Sample All Layers for the wand and Quick Selection; and
Auto-Enhance for Quick Selection. Quick Selection SHALL offer New, Add, and
Subtract only, with no Intersect mode.

#### Scenario: The wand options bar exposes the engine parameters
- **WHEN** the Magic Wand tool is active
- **THEN** the options bar shows combine mode, Tolerance, Contiguous, Anti-alias, and Sample All Layers

#### Scenario: Contiguous switches the wand mode
- **WHEN** Contiguous is unchecked and the wand clicks a colour region
- **THEN** every matching pixel in the image is selected, not only the connected region

#### Scenario: Modelled and unmodelled options are distinct [lst_options_aa]
- **WHEN** the wand, lasso, ellipse, or Quick Selection options bar is shown
- **THEN** Anti-alias is enabled for the ellipse, lasso, and wand tools, while Sample All Layers and Auto-Enhance remain disabled and carry a reason

#### Scenario: Unmodelled options are disabled with a reason
- **WHEN** the wand or Quick Selection options bar is shown
- **THEN** Sample All Layers and Auto-Enhance are disabled and carry a reason

#### Scenario: Quick Selection has no Intersect
- **WHEN** the Quick Selection options bar is shown
- **THEN** only New, Add, and Subtract are present

## ADDED Requirements

### Requirement: Anti-aliased lasso rasterisation
The system SHALL apply the tool-time Anti-alias setting when the Lasso,
Polygonal Lasso, or Magnetic Lasso commits its closed polygon. With Anti-alias
on, the polygon boundary SHALL be rasterised with fractional coverage so that
boundary pixels MAY hold values strictly between 0 and 255. With it off, every
coverage value SHALL be 0 or 255. Anti-aliasing SHALL be applied to the new
primitive before the combine mode and Feather. The 50 %-coverage contour of an
anti-aliased lasso mask SHALL follow the drawn polygon within 1 px.

#### Scenario: Lasso Anti-alias produces fractional coverage [lst_aa_lasso]
- **WHEN** a freehand Lasso is drawn with Anti-alias on and then again with it off
- **THEN** the anti-aliased mask contains boundary values strictly between 0 and 255, while the mask with Anti-alias off contains only 0 and 255

#### Scenario: Polygonal Lasso Anti-alias follows the clicked polygon [lst_aa_polygon]
- **WHEN** the Polygonal Lasso closes a polygon with Anti-alias on
- **THEN** boundary pixels hold intermediate coverage and the 50 %-coverage contour sits within 1 px of the clicked polygon
