## MODIFIED Requirements

### Requirement: Shadows/Highlights adjustment

The system SHALL use the full input layer luminance to compute independent smooth shadow and highlight weights for an expanded Shadows/Highlights adjustment. Both bands SHALL have Amount 0–100%, Tonal Width 0–100%, and Radius 0–500 px. Increasing width SHALL expand the affected range from the corresponding tonal extreme toward and into the midtones; 50% SHALL split the bands near middle gray and 100% SHALL include midtones without affecting the opposite extreme. It SHALL provide Color Correction or grayscale Brightness and Midtone Contrast, each from −100 to +100, and Black Clip and White Clip at 0–10% in increments of 0.01%. Positive Color Correction SHALL increase chroma in adjusted color areas and negative values SHALL decrease it; positive grayscale Brightness SHALL lighten and negative values SHALL darken. Neutral settings SHALL preserve all samples. Invalid parameters SHALL be rejected before any write. Existing four-byte `shdH` settings SHALL retain their former pointwise behavior. Expanded settings SHALL use a distinct versioned `shd2` block and round-trip with all values preserved. The encoding SHALL represent amounts, widths, radii and signed correction values in whole units and clip percentages in 0.01% increments; the encoder SHALL reject values outside those increments. Unsupported versions and malformed expanded blocks SHALL be rejected without applying or rewriting them as legacy settings.

#### Scenario: Matching pixels have different surrounds

- **WHEN** equally bright shadow pixels have different local neighborhoods and the shadow amount and radius are positive
- **THEN** the darker neighborhood produces more lift and a radius of zero makes the two corrections equal

#### Scenario: Advanced correction and clipping

- **WHEN** the user increases local or global correction on a suitable image
- **THEN** widths 0%, 50%, and 100% progressively expand the affected tonal range, the highlight amount darkens highlights, positive midtone contrast increases tonal separation, and nonzero clipping extends eligible endpoints without division by zero on a flat image
- **AND** positive and negative Color Correction respectively increase and decrease chroma in adjusted color areas, while positive and negative grayscale Brightness respectively lighten and darken

#### Scenario: Precision and parameter compatibility

- **WHEN** an expanded operator is applied to 8- or 16-bit samples
- **THEN** neutral settings preserve color and alpha, 16-bit adjustment retains precision, and invalid parameters leave the input unchanged
- **AND** non-neutral legacy `shdH` blocks produce their former pointwise output at both depths, representable `shd2` settings retain all fields, and nonrepresentable values, unsupported versions or malformed expanded blocks are rejected

### Requirement: Shadows/Highlights image dialog

The Image menu SHALL expose a destructive adjustment for supported pixel layers. Its dialog SHALL start with the two Amount controls and offer Show More Options to reveal the remaining controls. It SHALL expose Brightness only for Grayscale and Color Correction for color. The preview SHALL use the same full-layer context as the final apply, selection SHALL limit the write, and a successful apply SHALL create one undoable history operation. The expanded operation SHALL be unavailable at 32 bpc.

#### Scenario: Disclosure and mode controls

- **WHEN** the user opens Shadows/Highlights in RGB or Grayscale and enables Show More Options
- **THEN** the eight advanced controls become visible, with Brightness in Grayscale and Color Correction in RGB, and edits to either control reach preview and commit

#### Scenario: Preview and selected commit

- **WHEN** the user previews and accepts settings with a partial selection
- **THEN** preview and commit agree, changes stay inside the selection, and one undo restores the original samples

#### Scenario: 32-bit refusal

- **WHEN** the active document uses 32 bits per channel
- **THEN** the command is disabled and applying an expanded block through the engine is refused
