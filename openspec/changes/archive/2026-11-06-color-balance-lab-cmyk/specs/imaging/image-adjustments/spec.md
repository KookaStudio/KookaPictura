## MODIFIED Requirements

### Requirement: Color Balance adjustment

The system SHALL implement `Adjustment::ColorBalance(ColorBalanceParams { shadows, midtones, highlights, preserve_luminosity })`. Each tonal band SHALL hold three shifts in the order cyan-red, magenta-green, yellow-blue, each defaulting to 0 and spanning `-100..=100`. Per pixel it SHALL compute `Y = 0.299*R + 0.587*G + 0.114*B`, weight the three bands with overlapping parabola windows that peak at `Y = 0`, `Y = 0.5`, and `Y = 1`, add the weighted per-channel shifts, and clamp. `preserve_luminosity` SHALL default to true and rescale the result toward the input luminance. A midtone-only shift SHALL leave black untouched. Oracle expectation: no faithful ImageMagick operator exists, so property tests cover the midtone red shift and luminosity preservation.

The adjustment SHALL run in the document's visible colour mode through `apply_in_mode` (and the native-depth `apply_native_in_mode`), selected from `Document::source_mode` or `Document::mode` by the render layer. The default mode SHALL be RGB, and the RGB path SHALL be byte-identical to the mode-less path.

- **Lab documents:** the system SHALL convert the working RGB to CIELAB D50, add the band-weighted Cyan–Red and Magenta–Green shifts to `a` (`a += cyanRed − magentaGreen`), subtract the Yellow–Blue shift from `b` (`b −= yellowBlue`), and convert back. `L` SHALL be unchanged, so Preserve Luminosity is satisfied by construction.
- **CMYK documents:** the system SHALL convert the working RGB to C/M/Y/K with no black generation (`C = 1 − R`, `M = 1 − G`, `Y = 1 − B`, `K = 0`), lower each named ink by its band-weighted shift scaled to the ink range (`C −= cyanRed / 255`, `M −= magentaGreen / 255`, `Y −= yellowBlue / 255`), hold `K`, and convert back. Preserve Luminosity SHALL apply the RGB Rec.601 renormalization to the result. The no-black model is an approximation: the working raster retains no K plate after an edit, and no profile-mediated CMYK conversion is claimed.
- **Dialog readout:** the Color Balance dialog SHALL show a read-only channel row above the bars for the active tone, using the document mode's channel names: `R/G/B` for RGB, `A/B` for Lab (`A = cyanRed − magentaGreen`, `B = −yellowBlue`), and `C/M/Y` for CMYK (the ink change, so the sign is inverted from the slider). The row SHALL follow the tone selector and the per-bar values.

#### Scenario: Midtone shift moves grey toward red

- **WHEN** a midtones red shift of +100 is applied to mid-grey
- **THEN** the red channel increases and black is unchanged

#### Scenario: Luminosity is preserved

- **WHEN** a midtone shift is applied with `preserve_luminosity` set to true to mid-grey
- **THEN** the output luminance is within 3 of the input

#### Scenario: RGB mode is the byte-identical default

- **WHEN** Color Balance is applied through `apply` or `apply_in_mode` with `ColorBalanceMode::Rgb`
- **THEN** both outputs are bit-identical to each other and to the pre-change kernel

#### Scenario: Lab mode moves the A and B channels

- **WHEN** a midtones Cyan–Red +40 shift is applied to mid-grey in `ColorBalanceMode::Lab`
- **THEN** the output Lab `a` increases, a Magenta–Green +40 shift decreases `a`, a Yellow–Blue +40 shift decreases `b`, and `L` stays within one unit of the input

#### Scenario: CMYK mode lowers the named ink

- **WHEN** a midtones Cyan–Red +100 shift is applied to mid-grey in `ColorBalanceMode::Cmyk`
- **THEN** the output is redder (the red channel exceeds green and blue), a Magenta–Green shift is greener, a Yellow–Blue shift is bluer, and the ink model's K value is unchanged

#### Scenario: The dialog readout follows the mode and tone

- **WHEN** the Color Balance dialog is opened on an RGB, Lab, or CMYK document
- **THEN** the readout row shows `R/G/B`, `A/B`, or `C/M/Y` respectively, updates when a bar moves, and resets to the selected tone's own triple

#### Scenario: Other adjustments ignore the mode

- **WHEN** an adjustment other than Color Balance is applied in `ColorBalanceMode::Lab` or `ColorBalanceMode::Cmyk`
- **THEN** its output is bit-identical to the RGB mode
