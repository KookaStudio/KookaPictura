# Spec Delta

## MODIFIED Requirements

### Requirement: Fill / Adjustment creation menu

The Layers panel's New Fill / Adjustment strip button SHALL open a menu whose
Fill group offers Solid Color…, Gradient…, and Pattern…. Solid Color and
Gradient SHALL create the matching fill layer through the bridge. Pattern SHALL
be enabled whenever a document is open, because the built-in pattern library is
always available, and SHALL NOT carry a no-patterns reason. The Adjustment group
SHALL list all sixteen CS6 adjustment kinds. Choosing an adjustment kind SHALL
create that adjustment layer through the bridge. The Fill and Adjustment groups
SHALL be separated as in CS6. The `Layer > New Fill Layer` submenu SHALL offer
the same three fill types with the same Pattern enablement.

#### Scenario: The menu lists every adjustment kind

- **WHEN** the New Fill / Adjustment menu is opened
- **THEN** it contains an entry for each of the sixteen adjustment kinds, each enabled, and the choosing creates the matching layer

#### Scenario: Pattern stays disabled until authoring exists

- **WHEN** the New Fill / Adjustment menu is opened and no pattern is available from the document or the built-in library
- **THEN** the Pattern… entry is present, disabled, and carries a no-patterns reason

#### Scenario: Pattern follows the document's pattern resources [lpr_pattern_entry]

- **WHEN** the menu is opened on a document with no embedded pattern resources
- **THEN** Pattern… is enabled because the built-in library is available, and choosing it opens the Pattern Fill editor with that library

#### Scenario: The Layer menu offers the fill types [lpr_layer_fill_menu]

- **WHEN** the `Layer > New Fill Layer` submenu is built
- **THEN** it offers Solid Color…, Gradient…, and Pattern… with the same enablement as the panel menu
