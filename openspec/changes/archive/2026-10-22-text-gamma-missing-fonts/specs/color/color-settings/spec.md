## ADDED Requirements

### Requirement: Color Settings exposes Blend Text Colors Using Gamma

The Color Settings dialog SHALL have a More Options button that reveals
Advanced Controls holding a Blend Text Colors Using Gamma checkbox and a gamma
value field (1.00–2.20, two decimals). Both SHALL open showing the stored
setting, default on at 1.45; the value field SHALL be disabled while the
checkbox is off. On OK the application SHALL store both as application
preferences, apply them to the compositor, and recomposite every open document
without recording a history state. On Cancel nothing SHALL change.

#### Scenario: The advanced controls default to on at 1.45 [cs_text_gamma_default]

- **WHEN** Color Settings is opened with no stored preference and More Options is pressed
- **THEN** the Blend Text Colors Using Gamma checkbox is checked, the value reads 1.45, and the controls are visible

#### Scenario: OK applies and recomposites [cs_text_gamma_apply]

- **WHEN** a document holds a type layer, the checkbox is cleared in Color Settings, and the dialog is accepted
- **THEN** the canvas composite of the type layer changes, the history count does not, and reopening the dialog shows the checkbox cleared

#### Scenario: The setting survives a restart [cs_text_gamma_persist]

- **WHEN** the gamma is set to off at 1.80 and the session is saved and reloaded
- **THEN** the loaded session reports off at 1.80
