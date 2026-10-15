## ADDED Requirements

### Requirement: Reduce Noise kind and dialog

The filter mapping SHALL recognise `reduce-noise` with twelve slots: composite
Strength and Preserve Details, Reduce Color Noise, Sharpen Details, Remove JPEG
Artifacts, an Advanced flag, then the per-channel Strength/Preserve pairs for R,
G and B. An empty slot list SHALL default to Strength 6 with every other control
off and every channel equal to the composite; a non-empty list of any other
length SHALL be refused. `Filter ▸ Noise ▸ Reduce Noise` SHALL open a dedicated
dialog with Basic and Advanced pages: Basic edits one Strength and Preserve
Details pair for the whole image, Advanced reveals a Channel menu that selects
Red, Green or Blue and edits that channel's pair, and the Advanced flag SHALL be
set exactly when the Advanced page is active.

#### Scenario: The kind resolves and defaults

- **WHEN** `reduce-noise` is resolved with an empty slot list
- **THEN** the filter carries Strength 6, every other control off, and each
  channel equal to the composite

#### Scenario: Wrong arity is refused

- **WHEN** `reduce-noise` is resolved with a slot list of the wrong length
- **THEN** no filter is produced and the command returns false without changing
  the document

#### Scenario: The menu opens the dedicated dialog and commits

- **WHEN** `Filter ▸ Noise ▸ Reduce Noise` is chosen on an editable pixel layer
- **THEN** the Basic/Advanced dialog opens, and on OK one history state named
  `Filter: Reduce Noise` is recorded with the mapped slots
