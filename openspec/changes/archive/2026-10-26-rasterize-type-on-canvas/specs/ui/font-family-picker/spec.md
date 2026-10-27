## ADDED Requirements

### Requirement: Colour faces list without a preview

The font family pickers (the Type options bar, the Character panel, the Glyphs
panel and the style dialogs) SHALL list a colour face, one carrying a `COLR`,
`CBDT`, `sbix` or `SVG ` table, by its name in the UI font with no preview. This
SHALL hold whether or not the face covers Latin letters. Other faces SHALL keep
their per-family preview.

#### Scenario: A popup opened onto a colour face renders promptly [ffp_color_fast]

- **WHEN** the family popup opens with a colour face current, preferring one that covers `A`
- **THEN** the popup renders within 150 ms

#### Scenario: The dropdown paints its rows promptly anywhere in the list [ffp_dropdown_items]

- **WHEN** the Type options bar's family dropdown opens at each page of the installed families
- **THEN** every popup paints its rows within 250 ms, not an empty frame

### Requirement: Family names read in the UI font

Each row of a font family picker SHALL draw the family name in the UI font,
followed by a short sample in the face itself: `Sample` for a face with Latin
letters, the face's own script sample otherwise, and none for a colour face or
a face that lacks the sample's glyphs. Every row SHALL be sized from the UI
font, so a thin, light, or dotted face reads as clearly as any other and opening
the list loads no face to measure it.

#### Scenario: A light face's name reads at full contrast [ffp_ui_font_names]

- **WHEN** the row for the lightest installed face (a Thin or ExtraLight weight) is drawn in the Type options bar's family picker
- **THEN** in the open popup its name reaches full text contrast
