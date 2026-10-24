# ui/type-preferences Specification

## Purpose

Provides the Edit > Preferences > Type pane settings that expose the East Asian
and Middle Eastern type options and Missing Glyph Protection, and makes those
settings govern the type panels and rendering. The placeholder-text preference
remains in `tools/type-placeholder-text`.

## ADDED Requirements

### Requirement: Type preferences page presents the type options and glyph protection

The Type preferences page SHALL present `Show Asian Text Options` (off by
default), `Middle Eastern` (off by default), and `Enable Missing Glyph
Protection` (on by default). The settings SHALL be process-wide: changing one
SHALL apply to every open document without recording a document history state.
The settings SHALL be saved with the session and restored on the next start,
with the glyph-protection value applied to the renderer before the first
document renders.

#### Scenario: Defaults [type_pref_defaults]

- **WHEN** the Type preferences page is opened without any prior change
- **THEN** Show Asian Text Options is off, Middle Eastern is off, and Enable Missing Glyph Protection is on

#### Scenario: A change applies to open documents

- **WHEN** the Middle Eastern setting is turned on while a type layer is open
- **THEN** the Paragraph panel exposes its direction control without recording a history state

#### Scenario: The settings survive a restart

- **WHEN** the three settings are changed and the session is saved and loaded again
- **THEN** each setting reads back its changed value and the renderer reports the saved glyph-protection value

### Requirement: Type settings govern the panel controls

`Show Asian Text Options` SHALL show the Character panel's Set Tsume control
while on and hide it while off. `Middle Eastern` SHALL show the Paragraph
panel's direction control while on and hide it while off. Hiding a control SHALL
NOT clear the attribute already stored on the layer.

#### Scenario: Tsume appears with Show Asian Text Options

- **WHEN** Show Asian Text Options is turned on with a type layer active
- **THEN** the Character panel shows Set Tsume with the layer's stored value

#### Scenario: Direction appears with Middle Eastern

- **WHEN** Middle Eastern is turned on with a type layer active
- **THEN** the Paragraph panel shows the LTR/RTL direction control with the layer's stored value

#### Scenario: Hiding keeps the stored value

- **WHEN** a layer's tsume is 50 and Show Asian Text Options is turned off and on again
- **THEN** the hidden control's value is preserved and the control returns showing 50

### Requirement: Missing Glyph Protection setting governs per-character fallback

While `Enable Missing Glyph Protection` is on, a character the active face does
not cover SHALL be shaped and rendered in a covering face. While it is off, the
active face SHALL be used unchanged. Toggling the setting SHALL re-render open
documents without recording a document history state.

#### Scenario: Turning protection off removes the substitution

- **WHEN** a layer contains a character outside its face's coverage and Missing Glyph Protection is turned off
- **THEN** the layer re-renders without a fallback face and no history state is recorded
