# Spec Delta

## MODIFIED Requirements

### Requirement: Type preferences page presents the type options and glyph protection

The Type preferences page SHALL present `Show Asian Text Options` (off by
default), `Middle Eastern` (off by default), and `Enable Missing Glyph
Protection` (on by default). The settings SHALL be process-wide: changing one
SHALL apply to every open document without recording a document history state.
The settings SHALL be saved in the preferences store and restored on the next start,
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
