# tools/select-menu Specification Delta

## ADDED Requirements

### Requirement: Live Select menu command surface

The system SHALL implement the CS6 Select menu rows Reselect, Inverse, the
Modify submenu (Border, Smooth, Expand, Contract, Feather), Grow, Similar, Save
Selection, Load Selection, All Layers, Deselect Layers, Similar Layers, Color
Range, Refine Edge, and Transform Selection through stable command ids and
bridge operations. Each row SHALL keep its documented menu path and shortcut.
Every row SHALL be a live command; Transform Selection SHALL be enabled only
when a non-empty selection exists.

#### Scenario: Implemented rows are enabled with a document [sm_surface]

- **WHEN** a document is open
- **THEN** Reselect, Inverse, Modify, Grow, Similar, Save Selection, Color Range, Refine Edge, and Transform Selection are present and live

#### Scenario: Transform Selection needs a selection [sm_transform_selection]

- **WHEN** a document is open with no active selection, or with an empty selection
- **THEN** Transform Selection is visible and disabled

## REMOVED Requirements

### Requirement: Select menu command surface

**Reason**: Superseded by *Live Select menu command surface*: all rows, including Transform Selection, are implemented now, so the deferred-row contract no longer applies.

**Migration**: Use `Select > Transform Selection` for the selection border; the row is enabled with a non-empty selection and disabled otherwise.
