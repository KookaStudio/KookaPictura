# Spec Delta

## MODIFIED Requirements

### Requirement: Family names read in the UI font

Each row of a font family picker SHALL draw the family name in the UI font,
followed by a short sample in the face itself: `Sample` for a face with Latin
letters, the face's own script sample otherwise, and none for a colour face or
a face that lacks the sample's glyphs. The sample's point size SHALL follow the
saved `Type > Font Preview Size` preference; `None` SHALL omit the sample. Every
row SHALL be sized from the UI font and the selected preview point size, so a
thin, light, or dotted face reads as clearly as any other, the sample is not
clipped, and opening the list loads no face to measure it.

#### Scenario: A light face's name reads at full contrast [ffp_ui_font_names]

- **WHEN** the row for the lightest installed face (a Thin or ExtraLight weight) is drawn in the Type options bar's family picker
- **THEN** in the open popup its name reaches full text contrast

#### Scenario: The preview size changes the sample [ffp_preview_size]

- **WHEN** `Type > Font Preview Size` is set to Huge and a family picker is opened
- **THEN** its sample is drawn larger than at Small and the row height grows so the sample is not clipped

#### Scenario: None omits the sample [ffp_preview_none]

- **WHEN** `Type > Font Preview Size` is set to None and a family picker is opened
- **THEN** each row draws only the family name in the UI font and no sample
