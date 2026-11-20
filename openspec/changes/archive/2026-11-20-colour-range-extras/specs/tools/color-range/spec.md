# Spec Delta

## MODIFIED Requirements

### Requirement: Color Range coverage

Color Range SHALL produce a graded coverage mask for one of: a set of sampled
colours (Fuzziness 0–200), a hue band (Reds … Magentas, excluding greys), a
tonal band (Highlights, Midtones, Shadows), Skin Tones, or Out Of Gamut. With
Sampled Colors, a pixel's coverage SHALL come from its best-matching sample, so
adding a sample widens the selection, and a sample without a document position
SHALL carry no proximity weight. With Localized Color Clusters on, each
sample's match SHALL additionally fall off with distance from where it was
sampled, and Range (0–100) SHALL set the falloff strength: lower Range keeps
matches near their sample, higher Range admits them farther away, and Range 100
SHALL apply no proximity penalty. Skin Tones SHALL select skin-chroma colours,
with Fuzziness widening the window and grey or saturated non-skin colours
excluded. Out Of Gamut SHALL select pixels the active proof setup cannot
reproduce. Fully transparent pixels SHALL take no coverage before Invert.
Raising Fuzziness SHALL never shrink a colour match; Invert SHALL yield
`255 − coverage`.

#### Scenario: Bands and fuzziness behave

- **WHEN** the `color_range` unit tests match red against red and blue, raise Fuzziness on a near miss and across a grey ramp, pick Reds / Greens against red, green, and grey, and pick Highlights / Shadows against white and black
- **THEN** the exact colour is fully selected and a far one not (reversed with Invert), more fuzziness takes more and partly, grey belongs to no colour band, and the tonal bands split light from dark

#### Scenario: Every sample contributes its best match [crx_samples]

- **WHEN** Sampled Colors is evaluated with red and blue samples over an image holding red, green, and blue pixels, then with a green sample added
- **THEN** red and blue are fully selected and green is not before the third sample, and all three are selected after it, while an unmatched colour stays unselected

#### Scenario: Localized clusters fall off with Range [crx_localized]

- **WHEN** a one-row image holds the sampled colour at both ends, one end is sampled, and Localized Color Clusters is evaluated at Range 0 and Range 100, with Fuzziness unchanged
- **THEN** the far end is unselected at Range 0 and selected at Range 100, while switching localization off selects both ends

#### Scenario: Skin tones ignore non-skin colours [crx_skin]

- **WHEN** Skin Tones is evaluated on a skin-coloured patch, a saturated blue patch, and a neutral grey patch, with Fuzziness raised
- **THEN** the skin patch is selected, blue and grey are not, and raising Fuzziness never removes the skin patch

#### Scenario: Out of gamut marks unprintable pixels [crx_gamut]

- **WHEN** Out Of Gamut is evaluated against the Working CMYK proof on a saturated colour the profile cannot reproduce and on a neutral grey it can
- **THEN** the saturated pixel is selected and the grey is not, and Invert reverses both

#### Scenario: Transparent pixels are not matched [crx_alpha]

- **WHEN** Sampled Colors is evaluated on a fully transparent pixel with Invert off and then on
- **THEN** the pixel is unselected with Invert off and selected with Invert on, the same result as an opaque pixel with no matching colour

### Requirement: Color Range dialog

Select > Color Range… SHALL open a non-modal dialog, unavailable for 32-bpc
documents. The dialog SHALL offer Sampled Colors, Skin Tones, the six hue
bands, the three tonal bands, and Out Of Gamut, which SHALL be offered only on
RGB documents; Fuzziness 0–200; Invert; Replace, Add, and Subtract eyedroppers;
Localized Color Clusters with Range 0–100, with Range disabled while
localization is off; and Selection and Image display previews, which Control or
Command SHALL toggle between. While an eyedropper is active a canvas click
SHALL sample the composite instead of reaching the active tool; Shift SHALL act
as Add and Alt as Subtract for that click. The dialog preview SHALL show the
mask OK would commit, intersected with any live selection. While the dialog is
open the image window SHALL preview the candidate selection in None, Grayscale,
Black Matte, White Matte, or Quick Mask mode, and previewing SHALL leave the
document and selection unchanged. OK SHALL make a new selection, or narrow a
live selection to its matching subset, as one "Color Range" state; Cancel SHALL
change nothing. Save and Load SHALL round-trip the dialog settings and sampled
colours through a settings file; a malformed or unsupported settings file SHALL
be refused without changing the dialog.

#### Scenario: Sample, preview, select, refine

- **WHEN** the `tst_color_range` test opens the dialog on a red square over white, clicks the canvas on the square, switches to Highlights and back, presses OK, cancels a second dialog, and runs it again on a live rectangle selection
- **THEN** the click samples red without a history state, the preview is white over the square and black elsewhere (Highlights the reverse, with the eyedropper off), OK records one "Color Range" state selecting only the square, Cancel records nothing, and the refine keeps only the red inside the rectangle

#### Scenario: Replace, add, and subtract samplers [crx_samplers]

- **WHEN** the dialog is opened on separate red and blue squares, red is clicked with Replace, blue is added, and blue is then Alt-clicked
- **THEN** the preview selects both squares after Add, selects only red after the subtract, and OK commits the remaining sample's coverage

#### Scenario: Localized clusters narrow the dialog preview [crx_dialog_localized]

- **WHEN** two separated red squares are shown, Localized Color Clusters is enabled after sampling one, and Range is lowered and then raised
- **THEN** Range is disabled while localization is off, the far square is unselected at the low Range and selected at the high Range once localization is on, and the Range control is clamped to 0–100

#### Scenario: Skin tones need no sample [crx_dialog_skin]

- **WHEN** Skin Tones is selected in the dialog
- **THEN** the colour swatch and eyedroppers are disabled, the preview selects skin-coloured pixels, and non-skin colours are excluded

#### Scenario: Out of gamut follows the proof setup [crx_dialog_gamut]

- **WHEN** the dialog is opened on an RGB document, a Grayscale document, and a CMYK document
- **THEN** Out Of Gamut is offered and evaluates against the proof setup only on RGB, and the item is absent on Grayscale and CMYK

#### Scenario: Selection and image display previews [crx_dialog_preview]

- **WHEN** the dialog's display preview is switched between Selection and Image with the control and with Control or Command
- **THEN** Selection shows the coverage mask, Image shows the composite, and the shortcut toggles the same pair with the same candidate settings

#### Scenario: The image window previews the selection [crx_canvas_preview]

- **WHEN** the dialog cycles the image-window Selection Preview through None, Grayscale, Black Matte, White Matte, and Quick Mask, then closes
- **THEN** each mode renders the described overlay over the unchanged composite, proof colours do not change the matched coverage, and closing restores the plain composite

#### Scenario: Save and load settings [crx_save_load]

- **WHEN** the settings and samples of a Sampled Colors configuration are saved to a file, the controls are changed, and the file is loaded
- **THEN** the select mode, Fuzziness, localization, Range, Invert, and sampled colours and positions are restored, and OK reproduces the same selection

#### Scenario: Invalid settings are refused [crx_save_load_invalid]

- **WHEN** a malformed or wrong-version settings file is loaded
- **THEN** the load is refused without changing the controls or the preview

#### Scenario: Preview reflects a live selection [crx_refine_preview]

- **WHEN** a live rectangle selection exists and the dialog previews a matching colour outside and inside it
- **THEN** the preview is clipped to the live selection and matches the intersection OK commits
