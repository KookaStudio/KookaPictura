# color-settings Specification

## Purpose
The incoming-profile policy that decides how an embedded profile is honoured on open, and its Color Settings UI.

## Requirements

### Requirement: An incoming-profile policy decides how an embedded profile is honoured on open

`pictura-color` SHALL expose `Policy { Off, Preserve, Convert }` for an opened
RGB document's embedded ICC profile, defaulting to `Preserve`. The policy SHALL
be an application preference, not a document or history state. `pictura-codec`
SHALL expose `read_psd_with(bytes: &[u8], policy: Policy) -> Result<Document,
PsdError>` that applies the policy when the document is RGB and its image
resources carry a resource `1039` that parses as a non-sRGB ICC profile:

- `Preserve` SHALL leave the composite and every layer's pixel bytes unchanged,
  SHALL keep resource `1039`, and SHALL set `Document.document_icc` to the
  embedded profile bytes so the display converts it to the sRGB working space.
- `Convert` SHALL convert the composite and every layer's color channels to sRGB
  with relative-colorimetric intent, record the embedded bytes in
  `Document.source_icc`, remove resource `1039`, and leave `document_icc` `None`.
- `Off` SHALL leave the pixel bytes unchanged, remove resource `1039`, and leave
  the document untagged (`document_icc` and `source_icc` both `None`).

A document that is not RGB, has no profile, carries an sRGB profile, or carries
an undecodable profile SHALL be unchanged under every policy. `read_psd(bytes)`
SHALL remain equivalent to `read_psd_with(bytes, Policy::Convert)` for callers
that do not supply a policy.

#### Scenario: Preserve keeps the embedded profile and its pixels

- **WHEN** an RGB file with a non-sRGB ICC profile is read with `Policy::Preserve`
- **THEN** the composite and layer pixels are byte-identical to the file's, resource `1039` still holds the embedded bytes, and `document_icc` is the embedded profile

#### Scenario: Convert matches the existing normalisation

- **WHEN** the same file is read with `Policy::Convert`
- **THEN** the pixels are the sRGB conversion (within one LSB of lcms2), resource `1039` is removed, `source_icc` holds the embedded bytes, and `document_icc` is `None`

#### Scenario: Off ignores the profile

- **WHEN** the same file is read with `Policy::Off`
- **THEN** the pixels are byte-identical to the file's, resource `1039` is removed, and the document is untagged

#### Scenario: A profile-less or sRGB file is unchanged

- **WHEN** a file with no profile or an sRGB profile is read under any policy
- **THEN** the pixels and resources are unchanged and `document_icc` is `None`

#### Scenario: A Grayscale file is unchanged

- **WHEN** a Grayscale file with a non-sRGB profile is read under any policy
- **THEN** the pixels and resources are unchanged

### Requirement: The policy is persisted and applied when the application opens a file

The application SHALL persist the RGB incoming-profile policy as an application
preference, default it to `Preserve`, and pass it to `read_psd_with` when it
opens a document, so an open honours the configured policy. Changing it SHALL NOT
create a history state.

#### Scenario: The default policy is Preserve

- **WHEN** the application is started with no stored preference
- **THEN** the reported incoming policy is Preserve

#### Scenario: A stored policy survives a restart

- **WHEN** the policy is set to Convert and the session is saved and reloaded
- **THEN** the reported incoming policy is Convert

#### Scenario: Opening uses the configured policy

- **WHEN** the policy is Preserve and a file with a non-sRGB embedded profile is opened
- **THEN** the opened document keeps the embedded profile (`document_icc` is set) rather than being converted

### Requirement: Color Settings exposes the incoming policy

The application SHALL enable `Edit > Color Settings…` whenever the application
is running and SHALL open a dialog that shows the RGB working space (sRGB) and
lets the user choose the incoming-profile policy (`Off` / `Preserve Embedded
Profiles` / `Convert to Working RGB`), defaulting to the stored value. On OK the
choice SHALL be persisted; on Cancel it SHALL be unchanged.

#### Scenario: The dialog edits the policy

- **WHEN** Color Settings is opened, the policy is changed to Off, and the dialog is accepted
- **THEN** the reported incoming policy is Off and the working space reads sRGB

#### Scenario: Cancelling changes nothing

- **WHEN** the policy is changed in the dialog and the dialog is rejected
- **THEN** the reported incoming policy is unchanged

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

#### Scenario: OK applies a gamma value [cs_text_gamma_value]

- **WHEN** a document holds a type layer and the gamma value is set to 1.80 in Color Settings and accepted
- **THEN** the type's edges darken relative to 1.45, the history count does not change, the stored value is 1.80, and reopening the dialog shows 1.80

#### Scenario: The setting survives a restart [cs_text_gamma_persist]

- **WHEN** the gamma is set to off at 1.80 and the session is saved and reloaded
- **THEN** the loaded session reports off at 1.80
