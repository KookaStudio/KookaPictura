# color/profile-warnings Specification

## Purpose
Decide how an opened document's embedded or missing ICC profile is handled before the document is committed, with an explicit user choice when the configured warnings are enabled.

## Requirements

### Requirement: Profile warnings are offered before an open is committed

When the matching ask preference is enabled, the application SHALL inspect an opened PSD/PSB, JPEG, TIFF, or PNG before committing the document and SHALL show a modal warning when the file's embedded profile differs from the active working space of its colour model, or when the file has no embedded profile. The mismatch warning SHALL offer `Use the embedded profile`, `Convert to working space`, and `Discard the embedded profile`; the missing-profile warning SHALL offer `Assign working space`, `Assign then convert`, and `Leave without a profile`. The chosen answer SHALL override the stored incoming policy for that open only and SHALL NOT create a history state. Cancelling or closing the warning SHALL abort the open without creating a document. A file whose embedded profile is sRGB, or whose profile cannot be decoded, SHALL open without a warning.

#### Scenario: A mismatch can keep the embedded profile

- **WHEN** an RGB PSD with an Adobe RGB profile is opened while the working RGB space is sRGB and mismatch warnings are on, and the user chooses Use the embedded profile
- **THEN** the document opens with its pixels unchanged, `document_icc` holds the Adobe RGB bytes, and no history state is added by the warning

#### Scenario: A mismatch can convert to the working space

- **WHEN** the same file is opened and the user chooses Convert to working space
- **THEN** the document opens with its pixels converted to sRGB, resource 1039 removed, `source_icc` holding the embedded bytes, and no warning-created history state

#### Scenario: A mismatch can discard the profile

- **WHEN** the same file is opened and the user chooses Discard the embedded profile
- **THEN** the document opens with its pixels unchanged, resource 1039 removed, and no working or source profile recorded

#### Scenario: A missing profile can be assigned the working space

- **WHEN** an untagged RGB JPEG is opened with missing-profile warnings on and the user chooses Assign working space
- **THEN** the document opens with sRGB as its working space and no embedded profile resource

#### Scenario: A missing profile can be assigned and converted

- **WHEN** an untagged RGB TIFF is opened with missing-profile warnings on and the user chooses Assign then convert
- **THEN** the document opens with sRGB as its working space and no embedded profile resource, and the pixels are left numerically unchanged because the assigned space is already the working space

#### Scenario: A missing profile can stay untagged

- **WHEN** an untagged PNG is opened with missing-profile warnings on and the user chooses Leave without a profile
- **THEN** the document opens untagged with no working or source profile recorded

#### Scenario: Cancelling the warning aborts the open

- **WHEN** a warning is shown and the user cancels or closes it
- **THEN** no document is created and the application remains on its previous document

#### Scenario: A CMYK mismatch uses the CMYK working space

- **WHEN** a CMYK PSD with an embedded CMYK profile is opened while the working CMYK space is Photocraft Coated CMYK and mismatch warnings are on
- **THEN** the warning compares the embedded profile with the working CMYK space and the chosen answer applies the matching CMYK policy

#### Scenario: Warnings are skipped when disabled

- **WHEN** a file with a mismatched or missing profile is opened and the matching ask preference is off
- **THEN** no warning is shown and the stored incoming policy is applied silently

### Requirement: Warning preferences default off and are application-wide

The application SHALL persist separate ask preferences for profile mismatch and missing profile, SHALL default both to off, and SHALL apply a change to subsequent opens without altering an already open document or creating a history state. The preferences SHALL be app-global, not per-document.

#### Scenario: The default is off

- **WHEN** the application starts with no stored warning preference
- **THEN** both ask preferences read off and opening an untagged or mismatched file shows no warning

#### Scenario: A stored preference survives a restart

- **WHEN** the mismatch ask preference is enabled, the session is saved, and the application is restarted
- **THEN** the mismatch ask preference reads enabled

#### Scenario: Changing a warning preference is not a document edit

- **WHEN** a warning preference is changed with a document open
- **THEN** the document's history count and dirty state are unchanged
