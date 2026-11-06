# Test Plan

Covers the `imaging/image-adjustments` delta for `color-balance-lab-cmyk`. The
two existing scenarios keep their tests; the new scenarios cover the mode
mapping and the dialog readout. No manual rows are expected.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| Midtone shift moves grey toward red | Apply a midtones Cyan–Red +100 shift to a black and a mid-grey pixel in RGB mode | Black is unchanged; mid-grey's red channel rises | `pictura-adjust::color_balance_midtone_red_and_luminosity` |
| Luminosity is preserved | Apply a midtones shift with `preserve_luminosity` to mid-grey | Output Rec.601 luminance is within 3 of the input | `pictura-adjust::color_balance_midtone_red_and_luminosity` |
| RGB mode is the byte-identical default | Apply the same Color Balance through `apply` and `apply_in_mode(Rgb)` to one buffer | The two outputs are bit-identical | `pictura-adjust::color_balance_rgb_mode_is_the_default` |
| Lab mode moves the A and B channels | Apply midtones Cyan–Red +40, Magenta–Green +40, and Yellow–Blue +40 separately to mid-grey in Lab mode; read the output's Lab | `a` rises, then falls, then stays; `b` falls for Yellow–Blue; `L` stays within one unit | `pictura-adjust::color_balance_lab_shifts_a_and_b`; `pictura-adjust::color_balance_lab_mode_matches_the_eight_bit_shift` |
| CMYK mode lowers the named ink | Apply midtones Cyan–Red, Magenta–Green, and Yellow–Blue +100 separately to mid-grey in CMYK mode | Red, green, and blue respectively become the largest channel; K is unchanged in the ink model | `pictura-adjust::color_balance_cmyk_lowers_the_named_ink` |
| The dialog readout follows the mode and tone | Open the Color Balance dialog on an RGB image, a Lab conversion, and a CMYK conversion; read `colorBalanceReadout`; move the bars and switch tones | The row reads `R/G/B`, `A/B`, and `C/M/Y`; `A = cyanRed − magentaGreen`, `B = −yellowBlue`, `C/M/Y` are the inverted slider values; a tone switch shows that tone's triple | `tst_image_adjustments::colorBalanceReadoutFollowsModeAndTone` |
| Other adjustments ignore the mode | Apply Invert (and one tonal adjustment) in Lab and CMYK modes | Both outputs equal the RGB-mode output bit for bit | `pictura-adjust::color_balance_other_adjustments_ignore_the_mode` |
