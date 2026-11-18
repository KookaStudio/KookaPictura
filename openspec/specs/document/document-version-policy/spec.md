# Document version policy

## Purpose

Protect native documents that require a newer reader and define feature-based PSD and PSB interchange with Photoshop CS6 and CC.

## Requirements

### Requirement: Private version metadata describes Pictura documents

Native application saves SHALL write `opDocumentVersion`, `minReaderVersion`, and `writerVersion` to a Pictura-owned image resource and mirror them in XMP. The version fields SHALL be unsigned 16-bit integers. Version-aware saves SHALL use the lowest supported version pair that represents the output features, starting at `1 / 1` for Pictura's current model. The native container header SHALL continue to use version 1 for PSD and 2 for PSB. Unsupported native header versions SHALL fail with the version identified.

The reader SHALL use the private resource as its version record and treat the XMP properties as a mirror. A damaged unrelated XMP packet SHALL survive a save byte-for-byte. In this case the private resource SHALL still carry the version record and the application SHALL report that the XMP mirror could not be written.

#### Scenario: Native saves write version metadata [dvp_stamp]
- **WHEN** Pictura saves a supported document as PSD and PSB
- **THEN** both outputs carry the current private version record and equivalent XMP values, and their native headers are 1 and 2 respectively

#### Scenario: Header determines native format [dvp_header]
- **WHEN** files with PSD and PSB headers are opened with their two native filename extensions swapped, and a file with an unsupported header version is opened
- **THEN** the valid files decode according to their headers, and the invalid file reports its unsupported version

#### Scenario: An unrelated malformed XMP packet survives [dvp_xmp_preserved]
- **WHEN** an otherwise editable foreign document carries a malformed XMP packet and is saved
- **THEN** the private version record is written, the packet's original bytes survive, and the user is told the mirror could not be written

### Requirement: Private reader requirements determine access

A completely parsed resource section with an absent Pictura version record and absent Pictura XMP properties SHALL be treated as legacy or foreign. A valid supported pair SHALL permit editing if the document's features permit it. A newer document version with a supported reader floor SHALL warn once per open and permit editing if the document's features permit it. A normal native save SHALL retain that newer private record and its version values. An incompletely framed resource section (including a recognizable truncated owned record or an opaque tail that might hide another owned record), even if its readable prefix contains a valid stamp, a newer reader floor, malformed owned version record, or conflicting or uncorroborated Pictura XMP version pair SHALL warn and open read-only. A missing XMP mirror with a valid private record SHALL be accepted. PSD resource 1057 values and Adobe product metadata SHALL have no effect on the private reader floor.

#### Scenario: Indeterminate resource framing restricts access [dvp_incomplete_resources]
- **WHEN** the resource section stops at an invalid frame, including a truncated owned resource or an opaque block hiding a potential owned resource, with or without a valid preceding stamp
- **THEN** the document opens read-only with a reason and version-aware rewrite is refused

#### Scenario: Legacy and supported versions remain editable [dvp_legacy_current]
- **WHEN** supported native files with no Pictura metadata or a valid supported pair are opened
- **THEN** they are eligible for editing without a version warning

#### Scenario: A newer compatible stamp survives an edit [dvp_future_writable]
- **WHEN** a file with a newer Pictura document version and a supported reader floor is opened, edited, and saved
- **THEN** it warns once for that open and the saved file retains the newer pair and unrelated opaque bytes

#### Scenario: Reader floor and inconsistent owned metadata restrict edits [dvp_private_read_only]
- **WHEN** a file requires a newer Pictura reader, carries a malformed owned record, or has conflicting or uncorroborated Pictura XMP version values
- **THEN** the open reports the reason and the document is read-only

### Requirement: Photoshop interchange depends on file features

A local PSD or PSB from CS6 or CC SHALL be eligible for editing and native save when Pictura can render its document features and safely retain its data after editing. The application SHALL report read-only compatibility when it detects unsupported data that affects appearance or makes layered rewrite unsafe. An Adobe product version, a 1057 writer or reader string, and opaque data known to be safe to preserve SHALL leave the document eligible for editing. Known inert resources such as EXIF SHALL remain editable and be preserved. Unclassified layer or global records that may affect rendering or rewrite, and dependencies whose safety under geometry changes, layer reorder, or deletion cannot be established, SHALL restrict the document to read-only. The application SHALL combine codec structural evidence with renderer support without introducing a codec dependency on the renderer.

Shipment SHALL require actual open and reopen checks using locally Photoshop-authored specimens: CS6, CC 14.x, CC 2015, and the current and previous supported desktop releases at verification time. Each release SHALL cover layered PSD and PSB with compatibility composites enabled and disabled; the specimen set SHALL include linked Smart Objects, artboards, and modern adjustments. Record exact builds, formats, settings, features, appearance, editability, and read-only outcomes. Missing required builds or cases block shipment unless a deliberate documented scope change revises this requirement. Additional versions and combinations are characterization only; claims SHALL be limited to verified cases.

#### Scenario: CC metadata alone leaves a file editable [dvp_cc_metadata]
- **WHEN** a supported document contains a newer Adobe writer and opaque metadata known to be safe
- **THEN** the file remains editable and a native save retains that data

#### Scenario: Unsupported appearance or rewrite features trigger fallback [dvp_cc_feature]
- **WHEN** native file data indicates an unsupported artboard, linked object, adjustment, unclassified critical layer or global record, or unproven geometry, reorder, or deletion dependency that prevents safe interchange
- **THEN** the application explains the compatibility restriction and opens read-only

#### Scenario: Native output reopens in Photoshop [dvp_adobe_reopen]
- **WHEN** a reviewer opens Photoshop-authored PSD and PSB samples in Pictura, makes an edit in eligible files, and reopens the resulting saves in the representative Photoshop releases
- **THEN** the files open, the intended edit and expected appearance are present, and supported layers and features remain editable. Samples requiring read-only mode are reported as such
