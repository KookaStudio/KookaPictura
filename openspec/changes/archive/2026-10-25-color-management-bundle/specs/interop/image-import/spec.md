# Spec Delta

## ADDED Requirements

### Requirement: Raster imports apply the embedded ICC profile

The application's raster decode edge SHALL extract the embedded ICC profile from a JPEG (`APP2` `ICC_PROFILE`), TIFF (tag `34675`), or PNG (`iCCP`) file before constructing the document, and SHALL classify it as sRGB or non-sRGB by its `Description` tag. A non-sRGB profile SHALL be applied under the incoming-profile policy for that colour model:

- `Preserve` SHALL keep the decoded colour bytes, set `Document.document_icc` to the embedded profile bytes, and keep the file's `source_icc` empty.
- `Convert` SHALL convert the decoded colour bytes from the embedded profile to the sRGB working space with the configured conversion options, set `source_icc` to the embedded bytes, and leave `document_icc` empty.
- `Off` SHALL keep the decoded colour bytes and leave both profile fields empty.

An untagged raster, an sRGB-tagged raster, or a raster whose profile cannot be decoded SHALL be handled as an untagged sRGB import with no profile fields set. A placed raster image SHALL be converted to the host document's sRGB working space before it is added as a layer, because the engine's working pixels are sRGB. The profile decision is the same for `Open` and for `Place`; the warning flow is specified in `color/profile-warnings`.

#### Scenario: A JPEG with an Adobe RGB profile is preserved

- **WHEN** a JPEG with an Adobe RGB profile is opened with the Preserve policy
- **THEN** the document's pixels equal the decoded RGB values, `document_icc` holds the Adobe RGB bytes, and the view reports a preserved embedded profile

#### Scenario: A TIFF with an Adobe RGB profile is converted

- **WHEN** a TIFF with an Adobe RGB profile is opened with the Convert policy
- **THEN** the document's pixels are the lcms2 conversion to sRGB, `source_icc` holds the embedded bytes, and `document_icc` is empty

#### Scenario: A PNG with a non-sRGB profile is handled by policy

- **WHEN** a PNG with a non-sRGB iCCP profile is opened with the Off policy
- **THEN** the document's pixels equal the decoded RGB values and both profile fields are empty

#### Scenario: An untagged raster imports as sRGB

- **WHEN** a JPEG, TIFF, or PNG with no embedded profile is opened
- **THEN** the document imports as sRGB with no profile fields set and no warning

#### Scenario: A malformed profile is ignored

- **WHEN** a raster file carries an ICC profile that does not parse
- **THEN** the import succeeds with the decoded pixels and no profile fields set

#### Scenario: A placed raster is converted to the host working space

- **WHEN** a non-sRGB raster is placed into an sRGB document
- **THEN** the placed layer's pixels are the lcms2 conversion to sRGB and the host document's history gains one Place state
