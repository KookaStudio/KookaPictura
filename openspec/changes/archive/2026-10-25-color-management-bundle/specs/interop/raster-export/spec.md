# Spec Delta

## ADDED Requirements

### Requirement: JPEG and TIFF Save As preserve the RGB working profile

When a document is saved through `File > Save As` to JPEG or TIFF, the application SHALL write the document's working RGB pixel values rather than normalising them to sRGB, and SHALL embed the working RGB profile in an APP2 `ICC_PROFILE` segment (JPEG) or tag `34675` (TIFF) when the Embed Color Profile option is on. With the option off, the file SHALL be written untagged while the pixels keep the same working RGB values. A document whose working pixels are sRGB SHALL write sRGB values and MAY embed or omit the sRGB profile according to the option. A CMYK source document's raster save SHALL write the sRGB working pixels and SHALL NOT embed the CMYK source profile; its CMYK profile remains in a PSD/PSB save. Export As and Quick Export SHALL remain sRGB-normalised and SHALL NOT offer the profile option.

#### Scenario: An Adobe RGB document saves as a tagged JPEG

- **WHEN** a document whose working profile is Adobe RGB is saved as JPEG with embedding on
- **THEN** the JPEG's decoded pixels match the document's Adobe RGB values and its APP2 profile parses as Adobe RGB

#### Scenario: An Adobe RGB document saves as an untagged JPEG

- **WHEN** the same document is saved as JPEG with embedding off
- **THEN** the JPEG has no ICC profile and its decoded pixels still match the Adobe RGB values

#### Scenario: A TIFF save carries the profile tag

- **WHEN** a document whose working profile is Adobe RGB is saved as TIFF with embedding on
- **THEN** tag `34675` contains the Adobe RGB bytes and the raster samples match the working values

#### Scenario: Export As stays sRGB

- **WHEN** an Adobe RGB document is exported through Export As as JPEG
- **THEN** the written pixels are the sRGB conversion and no embedding option is offered

#### Scenario: A CMYK source document is not mis-tagged

- **WHEN** a CMYK PSD is saved as TIFF
- **THEN** the TIFF holds the sRGB working pixels and no CMYK profile
