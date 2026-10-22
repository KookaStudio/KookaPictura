# Spec Delta: ui/rulers-and-guides

## ADDED Requirements

### Requirement: Guide and ruler toggles skip loading documents

A guide or ruler toggle, and every refresh of the shared guide view state, SHALL
skip a document entry whose canvas or canvas host has not been created yet (a
document still loading) and SHALL NOT dereference that null pointer. The toggle
SHALL still update the persisted state. When the open completes and the canvas
is created, the new canvas SHALL receive the current ruler, guide, and
smart-guide state.

#### Scenario: Toggling while a document loads does not crash

- **WHEN** an open is held in flight with `setOpenDelayForTest` and the user dispatches `View > Rulers` and `View > Show > Guides`
- **THEN** no crash occurs, both flags change, and the completed document shows rulers and guides in the new state
