# Spec Delta

## ADDED Requirements

### Requirement: Built-in pattern library

`pictura-codec` SHALL expose `builtin_patterns()`, a small deterministic library
of project-authored opaque RGB patterns available to every document. The
library SHALL NOT contain Adobe preset patterns. The app SHALL expose the
document's decoded patterns first and then the built-in patterns whose ids are
not already present, so a document pattern with the same id wins. When a pattern
fill layer references a built-in pattern that the document does not carry, the
app SHALL encode that pattern and append it to the document's `Patt` block
before setting the layer's `PtFl` block, in the same edit, so the compositor,
the Layers thumbnail, and the saved PSD resolve the pattern.

#### Scenario: Built-ins are available without a document pattern [pat_builtin_available]

- **WHEN** a document has no `Patt`/`Pat2`/`Pat3` resource
- **THEN** the app still lists at least one available pattern id and the Pattern Fill editor is not empty

#### Scenario: A built-in is embedded when a fill references it [pat_builtin_embed]

- **WHEN** a pattern fill layer is created or edited with a built-in pattern id that the document does not carry
- **THEN** the document's `Patt` block gains that pattern in the same edit, the fill composites the tiled pattern, and the document round-trips with the pattern
