# imaging/image-size-controls Specification

## Purpose

Image Size resampling, style scaling, and print controls.

## Requirements

### Requirement: Image Size interpolation and precision

Image Size SHALL offer distinct Bicubic Smoother and Sharper methods. Automatic SHALL select Sharper for reduction, Smoother for enlargement, and ordinary Bicubic for an unchanged axis. Dense and tiled resampling SHALL agree on method behavior. When resampling retained float samples, Image Size SHALL preserve finite extended-range input in interpolation.

#### Scenario: Direction chooses each cubic mode [resize_methods]

- **WHEN** Automatic resizes an image with one axis shrinking and the other enlarging
- **THEN** it uses the Sharper and Smoother weights respectively, and the explicit methods produce distinct edge responses from Bicubic.

#### Scenario: Native floats retain extended range [resize_hdr]

- **WHEN** Image Size resamples retained float layer and document planes containing negative and greater-than-one values
- **THEN** the resulting samples incorporate those values without clipping to 0–1, and the planes remain tiled.

### Requirement: Image Size scales styles with a proportional resize

Image Size SHALL offer Scale Styles when Resample and Constrain Proportions are enabled. A proportional resize with Scale Styles selected SHALL scale all stored supported layer effects, including disabled effects, within the same undoable edit, while leaving the existing enabled-only behavior of Layer > Layer Style > Scale Effects unchanged. A target is proportional if the positive scale-factor intervals for both axes intersect: for source extent `s` and destination extent `d > 1`, the interval is `[(d − 0.5) / s, (d + 0.5) / s)`; for `d = 1`, it is `(0, 1.5 / s)`. This rule applies to constrained edits and Fit To targets. The scale factor SHALL be the destination/source ratio on the longer source axis, and an identity resize SHALL NOT scale styles. The recorded option SHALL be respected on replay, and an older action without the option SHALL keep the prior unscaled behavior.

#### Scenario: Style choice travels with the resize [resize_styles]

- **WHEN** an image with a layer shadow is enlarged proportionally with Scale Styles checked, the change is undone, and the resize is repeated with the option disabled
- **THEN** the shadow size doubles only during the first resize and Undo restores the original size; a stored disabled effect also scales and retains its scaled size when enabled.

#### Scenario: Rounded constrained targets scale styles [resize_styles_rounding]

- **WHEN** a 3 × 2 image is resized with Scale Styles selected to 5 × 3, or a target with a one-pixel extent is selected
- **THEN** style scaling follows the common rounding intervals and longer-axis ratio, while identity and targets with no common interval, including Fit To targets, leave styles unchanged.

### Requirement: Print resolution and physical units

Image Size SHALL calculate resolution from a positive screen frequency at Draft (1× with a 72 ppi floor), Good (1.5×), or Best (2×) and SHALL keep opening values on Auto Resolution cancel. Image Size SHALL offer Picas and offer Columns for width, deriving column measurements from persisted preference width and gutter. These units SHALL follow existing physical size coupling when Resample is off.

#### Scenario: Auto Resolution sets print resolution [resize_auto_ppi]

- **WHEN** the user accepts screen frequency 150 lines per inch at Best, or enters a low frequency with Draft, and tries canceling a subsequent Auto Resolution edit
- **THEN** resolution is 300 ppi, then at least 72 ppi, and Cancel keeps the current resolution.

#### Scenario: Physical dimensions use picas and configured columns [resize_print_units]

- **WHEN** width is set in Columns after setting width and gutter preferences, or a pica height is entered with resampling off
- **THEN** resolved pixel width accounts for interior gutters and physical height adjusts resolution without changing the existing pixel count.
