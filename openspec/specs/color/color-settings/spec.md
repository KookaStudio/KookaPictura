# color-settings Specification

## Purpose
The incoming-profile policy that decides how an embedded profile is honoured on open, and its Color Settings UI.

## Requirements

### Requirement: An incoming-profile policy decides how an embedded profile is honoured on open

`pictura-color` SHALL expose `Policy { Off, Preserve, Convert }` for an opened
RGB or CMYK document's embedded ICC profile, defaulting to `Preserve`. The policy
SHALL be an application preference, not a document or history state. The
application's RGB working space SHALL remain sRGB. `pictura-codec` SHALL expose
`read_psd_with_options(bytes: &[u8], options: &OpenOptions) -> Result<Document,
PsdError>`, where `OpenOptions` carries the policy, an optional working RGB
profile (sRGB when absent), an optional working CMYK profile (the bundled
Photocraft Coated CMYK when absent), and the `ConversionOptions` used by any
policy conversion. `read_psd_with(bytes, policy)` SHALL remain the library
convenience for callers that supply no working profiles.

For an RGB document carrying a resource `1039` that parses as a non-sRGB ICC
profile:

- `Preserve` SHALL leave the composite and every layer's pixel bytes unchanged,
  SHALL keep resource `1039`, and SHALL set `Document.document_icc` to the
  embedded profile bytes so the display converts it to the sRGB working space.
- `Convert` SHALL convert the composite and every layer's color channels to sRGB
  with relative-colorimetric intent and the configured options, record the
  embedded bytes in `Document.source_icc`, remove resource `1039`, and leave
  `document_icc` `None`.
- `Off` SHALL leave the pixel bytes unchanged, remove resource `1039`, and leave
  the document untagged (`document_icc` and `source_icc` both `None`).

For a CMYK document, the composite and layer CMYK planes are normalized to the
engine's RGB working mode, and the retained source planes remain available for a
save:

- `Preserve` SHALL decode the retained CMYK planes through the embedded CMYK
  profile, or through the working CMYK profile when the file has no embedded
  profile, keep resource `1039` as the document's CMYK profile, record no
  `source_icc`, and leave the retained CMYK planes unchanged. The normalized
  pixels are sRGB, so `document_icc` SHALL be `None`.
- `Convert` SHALL decode the retained CMYK planes, then re-encode them through a
  CMYK-to-CMYK transform from the embedded profile (or the working CMYK profile
  when the file is untagged) to the working CMYK profile, replace resource `1039`
  with the working CMYK profile, record the embedded bytes (when present) in
  `source_icc`, and set the working RGB to the decode of the re-encoded planes.
- `Off` SHALL decode the retained CMYK planes with the documented profile-free
  approximation, remove resource `1039`, and leave the document untagged.

The library convenience `read_psd_with` SHALL use the embedded CMYK profile when
present and the documented profile-free fallback for an untagged CMYK file, so a
caller with no working CMYK profile keeps the historical behaviour. A Grayscale
document, an RGB document with no profile or an sRGB profile, or an RGB document
with an undecodable profile SHALL be unchanged under every policy. `read_psd(bytes)`
SHALL remain equivalent to `read_psd_with(bytes, Policy::Convert)` for callers
that do not supply a policy.

#### Scenario: Preserve keeps the embedded profile and its pixels

- **WHEN** an RGB file with a non-sRGB ICC profile is read with `Policy::Preserve`
- **THEN** the composite and layer pixels are byte-identical to the file's, resource `1039` still holds the embedded bytes, and `document_icc` is the embedded profile

#### Scenario: Convert matches the existing normalisation

- **WHEN** the same RGB file is read with `Policy::Convert`
- **THEN** the pixels are the sRGB conversion within one LSB of lcms2, resource `1039` is removed, `source_icc` holds the embedded bytes, and `document_icc` is `None`

#### Scenario: Off ignores the profile

- **WHEN** the same RGB file is read with `Policy::Off`
- **THEN** the pixels are byte-identical to the file's, resource `1039` is removed, and the document is untagged

#### Scenario: A profile-less or sRGB file is unchanged

- **WHEN** an RGB file with no profile or an sRGB profile is read under any policy
- **THEN** the pixels and resources are unchanged and `document_icc` is `None`

#### Scenario: A Grayscale file is unchanged

- **WHEN** a Grayscale file with a non-sRGB profile is read under any policy
- **THEN** the pixels and resources are unchanged

#### Scenario: CMYK Preserve keeps the embedded profile and retained planes

- **WHEN** a CMYK file with an embedded CMYK profile is read with `Policy::Preserve` and the working CMYK profile is Photocraft Coated CMYK
- **THEN** the working RGB pixels are the profile-aware decode of the retained CMYK planes, resource `1039` still holds the embedded CMYK profile, the retained CMYK planes are byte-identical, `document_icc` is `None`, and `source_icc` is `None`

#### Scenario: CMYK Convert re-encodes into the working CMYK profile

- **WHEN** the same CMYK file is read with `Policy::Convert`
- **THEN** the retained CMYK planes are the CMYK-to-CMYK conversion into the working CMYK profile, resource `1039` holds the working CMYK profile, `source_icc` holds the embedded bytes, and the working RGB is the profile-aware decode of the re-encoded planes

#### Scenario: CMYK Off uses the profile-free fallback

- **WHEN** the same CMYK file is read with `Policy::Off`
- **THEN** the working RGB is the documented profile-free CMYK-to-RGB conversion, resource `1039` is removed, and the document is untagged

#### Scenario: An untagged CMYK file uses the working profile

- **WHEN** a CMYK file with no embedded profile is opened by the application with `Policy::Preserve` or `Policy::Convert`
- **THEN** the working RGB is decoded through the working CMYK profile and resource `1039` carries that working profile after Convert

#### Scenario: The library convenience keeps the profile-free fallback for untagged CMYK

- **WHEN** `read_psd_with` is called on an untagged CMYK file with no working CMYK profile supplied
- **THEN** the pixels use the documented profile-free fallback

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
is running and SHALL open a dialog that shows the fixed sRGB RGB working space;
a CMYK working-space picker whose default is Photocraft Coated CMYK; separate
RGB and CMYK incoming-profile policy pickers (`Off` / `Preserve Embedded
Profiles` / `Convert to Working RGB` or `Convert to Working CMYK`); the profile
mismatch and missing-profile ask toggles; and the conversion options (engine
`Little CMS 2`, rendering intent, black-point compensation, and dither). The
dialog SHALL keep the existing More Options button and its Blend Text Colors
Using Gamma controls. The dialog SHALL open showing the stored values. On OK the
choices SHALL be persisted; on Cancel no preference SHALL change.

#### Scenario: The dialog edits the policy

- **WHEN** Color Settings is opened, the RGB policy is changed to Off, and the dialog is accepted
- **THEN** the reported RGB incoming policy is Off and the RGB working space reads sRGB

#### Scenario: The dialog edits the CMYK policy

- **WHEN** the CMYK policy is changed to Preserve Embedded Profiles and the dialog is accepted
- **THEN** the reported CMYK incoming policy is Preserve Embedded Profiles

#### Scenario: The dialog edits the conversion options

- **WHEN** the intent is changed to Perceptual, black-point compensation is cleared, and the dialog is accepted
- **THEN** the stored conversion options report Perceptual with black-point compensation off

#### Scenario: The ask toggles are persisted

- **WHEN** the mismatch ask toggle is enabled and the dialog is accepted
- **THEN** reopening Color Settings shows the mismatch ask toggle enabled

#### Scenario: Cancelling changes nothing

- **WHEN** any policy, working space, ask toggle, or conversion option is changed in the dialog and the dialog is rejected
- **THEN** every reported setting is unchanged

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

### Requirement: Conversion options are persisted and applied

The application SHALL persist the rendering intent, black-point-compensation
flag, and dither flag as application preferences separate from any document, and
SHALL apply them to the incoming Convert policy, Convert to Profile, and the
picker's working-CMYK evaluation. Changing them SHALL NOT create a history state
or mark a document dirty; it SHALL affect subsequent conversions only.

#### Scenario: The defaults match CS6

- **WHEN** the application starts with no stored conversion preference
- **THEN** the reported intent is Relative Colorimetric, black-point compensation is on, and dither is on

#### Scenario: A stored option survives a restart

- **WHEN** the intent is set to Perceptual and the session is saved and reloaded
- **THEN** the reported intent is Perceptual

#### Scenario: Changing an option is not a document edit

- **WHEN** an option is changed with a document open
- **THEN** the document's history count and dirty state are unchanged
