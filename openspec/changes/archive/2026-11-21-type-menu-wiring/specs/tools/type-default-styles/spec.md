# Spec Delta

## Purpose

Save a type layer's character and paragraph attributes as profile defaults and apply those defaults to another type layer.

## ADDED Requirements

### Requirement: Save Default Type Styles

`Type > Save Default Type Styles` SHALL save the current type layer's character and paragraph attributes, including its font family and style, as the profile's default type style. It SHALL record no document history state, SHALL persist the defaults in the typed preferences store, and SHALL be disabled without a type layer.

#### Scenario: Saving captures the layer's style [tds_save]

- **WHEN** `Type > Save Default Type Styles` runs on a styled type layer
- **THEN** the layer's font family, style, character attributes, and paragraph attributes become the profile defaults, no document history state is recorded, and the layer itself is unchanged

#### Scenario: Saving needs a type layer [tds_save_disabled]

- **WHEN** the current layer is not a type layer
- **THEN** `Save Default Type Styles` is disabled

### Requirement: Load Default Type Styles

`Type > Load Default Type Styles` SHALL apply the saved default character and paragraph attributes to the current type layer, keep the layer's text, placement, and orientation, record exactly one `Load Default Type Styles` state, and be disabled before defaults have been saved, when the current layer is not a type layer, when the document's type layers cannot be re-set, or while a Type tool session is open.

#### Scenario: Loading applies the defaults [tds_load]

- **WHEN** defaults have been saved and `Type > Load Default Type Styles` runs on another type layer
- **THEN** the layer takes the saved font and attributes, keeps its own text and placement, and one state is recorded

#### Scenario: Undo restores the layer's style [tds_load_undo]

- **WHEN** the load is undone once
- **THEN** the layer's previous character and paragraph attributes return and its text is unchanged

#### Scenario: Loading before saving is disabled [tds_no_defaults]

- **WHEN** no defaults have been saved in the profile
- **THEN** `Load Default Type Styles` is disabled

#### Scenario: Loading needs an editable document [tds_not_editable]

- **WHEN** the document's type layers cannot be re-set or a Type tool session is open
- **THEN** `Load Default Type Styles` is disabled and records no state

### Requirement: Defaults survive a restart

The saved default type style SHALL be stored as a Type preference and restored on the next start, so `Load Default Type Styles` works without saving again.

#### Scenario: The defaults persist [tds_persist]

- **WHEN** defaults are saved and the application is relaunched
- **THEN** `Load Default Type Styles` is enabled and applies the same font and attributes as before the restart
