## MODIFIED Requirements

### Requirement: Select menu command surface

The system SHALL implement the CS6 Select menu rows Reselect, Inverse, the
Modify submenu (Border, Smooth, Expand, Contract, Feather), Grow, Similar, Save
Selection, Load Selection, All Layers, Deselect Layers, and Similar Layers
through stable command ids and bridge operations. Each row SHALL keep its
documented menu path and shortcut. The rows Color Range and Refine Edge SHALL be
implemented commands; the row Transform Selection SHALL remain present but
disabled with a documented reason.

#### Scenario: Implemented rows are enabled with a document

- **WHEN** a document is open with an active selection
- **THEN** Reselect, Inverse, Modify, Grow, Similar, Save Selection, Color Range, and Refine Edge are enabled

#### Scenario: Deferred rows stay disabled

- **WHEN** the Select menu is opened
- **THEN** Transform Selection is visible and disabled
