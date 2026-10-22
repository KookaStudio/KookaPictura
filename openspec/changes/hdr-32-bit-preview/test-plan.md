# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Exposure and Gamma changes only the display | Open a 32-bit fixture, note the display pixel, change the preview exposure, and compare the display, composite, and history | The display changes; the composite, retained samples, and history index do not | `tst_hdr_preview::exposureChangesDisplayOnly`, `pictura-app::hdr_preview_level0_changes_only_the_frame` |
| Highlight Compression changes only the display | Switch the preview method to Highlight Compression | The frame is rebuilt and the document remains unmodified | `tst_hdr_preview::highlightChangesDisplayOnly` |
| Reset restores the stored frame | Change the preview, then Reset | The frame matches the identity-transform frame | `tst_hdr_preview::resetRestoresStoredFrame` |
| An edited document previews its current source | Edit a 32-bit document, then change the preview | The frame is built from the edited composite, not the stale open-time store | `tst_hdr_preview::editedDocumentUsesCurrentSource`, `pictura-app::hdr_preview_uses_edited_composite` |
| A non-32-bit document cannot preview | Open an 8-bit document and inspect the View command | The command is disabled and no transform runs | `tst_hdr_preview::non32BitCommandDisabled` |
| The method gates the numeric fields | Cycle the dialog's Method control | Exposure/Gamma fields enable only for Exposure and Gamma | `tst_hdr_preview::methodGatesNumericFields` |
| The defaults are neutral | Open the dialog on a 32-bit document | Method is Exposure and Gamma, Exposure `0`, Gamma `1.0` | `tst_hdr_preview::defaultsAreNeutral` |
| Accept and cancel | Accept changed fields, then cancel after another change | Accept applies the new preview; cancel keeps the previous one | `tst_hdr_preview::acceptAndCancel` |
| The status-bar slider is per-view | Move the 32-Bit Exposure slider, reopen the document, and double-click the slider | The view exposure changes, the file is unaffected, and double-click resets to `0` | `tst_hdr_preview::statusBarExposureIsPerView` |
| `info_bit_depth_16` | Set the top-left block to 16-bit over white | R is `65535`, footer `16-bit` | `tst_info_panel::bitDepthScalesRgb` |
| `info_bit_depth_independent` | Set a CMYK block to 16-bit and then 32-bit | C/M/Y/K stay percentages; only the footer changes | `tst_info_panel::cmykDepthIndependent` |
| `info_bit_depth_32_float` | Open a 32-bit fixture with a current sample above `1.0`, set the block to 32-bit, and hover the sample | The unclamped float value is shown with three decimals | `tst_info_panel::hdrReadoutShowsUnclampedSamples` |
| `info_bit_depth_32_sampler` | Place a colour sampler over an unclamped 32-bit pixel at 32-bit | The sampler row uses the float source and its sample-size average | `tst_info_panel::hdrSamplerUsesFloatSource` |
| `info_bit_depth_32_fallback` | Report 32-bit depth with no current float source and hover a pixel | The readout uses `v / 255.0` and the panel does not crash | `tst_info_panel::hdrReadoutFallsBackWithoutFloatStore` |
| HDR mode opens for a 32-bit document | Click the foreground swatch with a 32-bit document active | The HDR picker opens with Intensity and float fields | `tst_color_picker::hdrModeOpensFor32Bit` |
| Intensity scales the linear colour | Set intensity to `+2` stops | Float RGB values are the base linear-light values times `4` | `tst_color_picker::intensityScalesLinearColor` |
| Float fields accept values above white | Enter `2.5`, `1.0`, `0.25` | The colour is the float triple, not clamped to `1.0` | `tst_color_picker::floatFieldsAcceptAboveWhite` |
| Non-finite input is clamped | Commit `NaN` or an infinite field | The value is clamped to the finite boundary and the colour stays finite | `tst_color_picker::nonFiniteInputIsClamped` |
| Preview swatches follow stop size | Set Preview Stop Size to `3` with Relative to Document off | Swatches appear at `−9, −6, −3, +3, +6, +9` stops | `tst_color_picker::previewSwatchesFollowStopSize` |
| Relative to Document folds in the preview | Turn Relative to Document on with a non-zero preview exposure | Each swatch includes the document exposure | `tst_color_picker::relativeToDocumentFoldsPreview` |
| Add to Swatches keeps the float value | Add an HDR colour to the Swatches panel and reopen it | The new swatch retains the float triple | `tst_color_picker::addToSwatchesKeepsFloat`, `tst_swatches_panel::hdrSwatchRoundTrips` |
| 8-bit documents keep the ordinary picker | Click a swatch with an 8-bit document active | The ordinary picker opens with no Intensity or float fields | `tst_color_picker::eightBitKeepsOrdinaryPicker` |
| Toolbox swatch | Click a toolbox foreground/background swatch and choose a colour | That swatch adopts the chosen colour | `tst_color_picker::toolboxSwatchAdoptsColour` |
| Color panel field | Click the Color panel field or ramp | The active foreground/background colour follows it | `tst_color_picker::colorPanelFieldSetsSwatch` |
| A 32-bit toolbox swatch opens the HDR picker | Click a toolbox swatch with a 32-bit document active | The HDR picker opens instead of the ordinary picker | `tst_color_picker::hdrModeOpensFor32Bit` |
| Add an HDR swatch | Add a colour above `1.0` from the HDR picker | The swatch stores the float triple | `tst_swatches_panel::hdrSwatchRoundTrips` |
| Click an HDR swatch | Click an HDR swatch | The foreground HDR state and its `QColor` mirror both restore | `tst_swatches_panel::hdrSwatchClickRestoresFloat` |
| HDR swatches persist | Add an HDR swatch and create a new window | The swatch still carries its float triple | `tst_session_store::hdrSwatchPersists` |
| Ordinary swatches are unchanged | Add and click an 8-bit swatch | It behaves exactly as before and carries no float triple | `tst_swatches_panel::ordinarySwatchUnchanged` |
