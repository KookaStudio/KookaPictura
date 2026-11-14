# ui/font-family-picker Specification

## Purpose
The font family pickers list every installed family and open promptly: plain
faces preview in their own font, and colour faces list by name only.

## Requirements

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

### Requirement: Typing a family name starts a new search

When a font family field is focused, the first printable character typed SHALL
start a new family search: it SHALL replace the family already shown rather than
append to it. Navigation and deletion keys SHALL leave the shown text editable
without starting a new search. A programmatic family write from the Type options
or a text session that carries the family already shown in the field SHALL NOT
replace the field's text or clear its completion; an external change to a
different family may update the field.

#### Scenario: A typed prefix replaces the shown family [ffp_typed_prefix]

- **WHEN** the Character panel's family field shows `Liberation Sans` and the user focuses it and types `noto`
- **THEN** the field's text is a family beginning with `noto` under a case-insensitive comparison, not `Liberation Sansnoto`

#### Scenario: Navigation keeps the shown family editable [ffp_nav_keeps_text]

- **WHEN** the Character panel's family field shows `Liberation Sans` and the user focuses it, presses End, and types ` x`
- **THEN** the field keeps `Liberation Sans` and appends ` x` (the text is edited, not replaced)

#### Scenario: A live option sync keeps the typed prefix [ffp_live_sync]

- **WHEN** the Type options bar or the Character panel writes the same family back while the user is typing a prefix
- **THEN** the field keeps the typed prefix and its completion set, and does not jump to an unrelated family

### Requirement: Family completion matches case-insensitively

The font family fields SHALL complete from the typed prefix against every
installed family whose name begins with that prefix, comparing
case-insensitively, so typing `noto` offers `Noto Sans` regardless of the
family's casing. Selecting a family from the family list SHALL keep applying
the exact family name.

#### Scenario: Lowercase prefixes match mixed-case families [ffp_prefix_case]

- **WHEN** `noto` is typed as the search prefix in a family field
- **THEN** the offered completions include `Noto Sans`

#### Scenario: Every family input completes the same way [ffp_all_inputs]

- **WHEN** `noto` is typed into the Character panel, the Type options bar, the Paragraph Style dialog, or the Glyphs panel family field
- **THEN** each field offers `Noto Sans` as a case-insensitive completion

#### Scenario: Exact-case selection is unchanged [ffp_exact_pick]

- **WHEN** the user selects `Noto Sans` from a family list
- **THEN** `Noto Sans` is applied exactly, as before
