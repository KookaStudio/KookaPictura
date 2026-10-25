# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Convert transforms the pixels | Convert a known color patch to Adobe RGB with Relative Colorimetric and BPC on | The composite equals the lcms2 reference conversion within one LSB | `pictura-codec::convert_srgb_to_adobe_matches_lcms2` |
| Convert preserves appearance and tags the destination | Convert sRGB to Adobe RGB, display it, then convert back to sRGB | The displayed bytes stay close and each destination resource is tagged correctly | `pictura-codec::convert_srgb_to_adobe_matches_lcms2`, `pictura-codec::convert_back_to_srgb_leaves_the_document_untagged` |
| Dither changes only 8-bit output | Run the same conversion with dither on and off on 8-bit and 16-bit documents | The 8-bit outputs differ deterministically; the 16-bit outputs agree | `pictura-color::dither_is_deterministic_and_8bit_only` |
| Flatten leaves one Background | Convert with Flatten Image on a two-layer document | Exactly one opaque Background remains and its pixels are the converted composite | `pictura-codec::convert_flatten_leaves_one_background` |
| A non-RGB document is refused | Apply Convert to a flat CMYK document | The call returns false and the document is unchanged | `pictura-codec::convert_refuses_non_rgb` |
| A source-mode native store is left alone | Read a 16-bit layered CMYK PSD carrying an embedded profile, then run Convert to Profile to sRGB | The 8-bit layer channels equal the converted composite; the retained CMYK store is not treated as RGB | `pictura-codec::convert_does_not_reinterpret_native_cmyk_layers_as_rgb` |
| Color panel swatch opens the picker | Click the Color panel's foreground swatch, choose `#0c2238` in the picker, and accept | The picker opens and the shared foreground becomes `#0c2238` | `tst_color_picker::colorPanelSwatchOpensPicker` |
