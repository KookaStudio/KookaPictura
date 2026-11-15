# codec/iptc-extension Specification

## Purpose
Reads and writes the IPTC Extension (Iptc4xmpExt) fields that the File Info dialog exposes, keeping the nested structures that are not yet editable visible and preserved.

## Requirements

### Requirement: IPTC Extension fields are read into labelled rows

`pictura-codec` SHALL read the document's XMP packet into labelled IPTC Extension rows for: Person Shown (`PersonInImage`, a bag of text), Organisation Shown name and code (`OrganisationInImageName` and `OrganisationInImageCode`, bags of text), Event (`Event`, a lang alternative), Location Created (`LocationCreated`, a struct of City, State/Province, Country Name, Country Code, Sublocation, and World Region), Location Shown (`LocationShown`, a struct with the same leaves), Image Rating (`ImageRating`, an integer in 1..=5), Model Release Status and Property Release Status (`ModelReleaseStatus`/`PropertyReleaseStatus`, text), Digital Source Type (`DigitalSourceType`, text), Additional Model Information (`AddlModelInfo`, text), and Licensor (`Licensor`, a struct of Name, ID, Email, Phone, City, State, Country, Postal Code, Street Address, and Web URL). A bag SHALL yield one row per item, a lang alternative SHALL yield its `x-default` value, and a struct SHALL yield one row per leaf. Read rows SHALL list only values present in the packet; editable rows are a separate surface built from the schema and SHALL NOT be inferred from this requirement. A malformed or truncated packet SHALL yield the rows decoded so far without panicking.

#### Scenario: The fixture's extension fields read

- **WHEN** the fixture's XMP packet is read
- **THEN** its person bag, event, location-created struct, and image rating are returned as labelled rows

#### Scenario: A missing extension namespace yields no rows

- **WHEN** the document's packet carries no Iptc4xmpExt property
- **THEN** the extension read rows are empty and reading does not fail

#### Scenario: A malformed extension packet does not panic

- **WHEN** the packet is truncated inside an extension structure
- **THEN** reading returns the rows decoded so far and does not panic

### Requirement: Supported IPTC Extension fields are editable

`pictura-codec` SHALL apply edits to the supported IPTC Extension fields in the document's XMP packet only, because IPTC Extension has no IIM channel. It SHALL create a missing simple property, bag, or struct, SHALL replace or remove existing leaves, SHALL write one item per value for bags and the `x-default` item for lang alternatives, and SHALL return whether anything changed. An edit that leaves every value unchanged SHALL return false. A property shape the writer cannot safely rewrite SHALL leave the packet unchanged for that field, and an apply that cannot write every requested field SHALL mutate nothing and report failure rather than applying a partial set. Every other image resource and unparsed tail SHALL be preserved byte-for-byte, and unknown extension properties SHALL survive verbatim.

#### Scenario: A person is added to the bag

- **WHEN** Person Shown is set to two values on a document with no Person Shown property
- **THEN** the packet carries a bag with both values and reading returns both rows

#### Scenario: A location struct leaf is edited

- **WHEN** the City leaf of Location Created is changed
- **THEN** the new value reads back from the same struct leaf and the other leaves are unchanged

#### Scenario: Clearing a field removes it

- **WHEN** an extension field is set to an empty value list
- **THEN** the property is absent when read again and the document remains well formed

#### Scenario: Unknown extension properties survive an edit

- **WHEN** an extension field is edited in a packet that also carries an unknown Iptc4xmpExt property
- **THEN** the unknown property's bytes are unchanged

#### Scenario: An unsupported field aborts the apply

- **WHEN** one requested field uses a shape the writer cannot safely rewrite
- **THEN** no requested field is applied, the packet is unchanged, and the operation reports failure

### Requirement: Unsupported nested structures are listed read-only

`pictura-codec` SHALL expose the leaves of Artwork or Object Shown and of any IPTC Extension property outside the supported set as labelled read-only rows. It SHALL NOT provide an edit path for those rows, and SHALL preserve their bytes on every write. When a packet carries more than one `LocationShown` or `Licensor` structure, the first SHALL be editable and every additional structure SHALL be listed read-only.

#### Scenario: Artwork leaves are listed without an edit path

- **WHEN** the packet carries an Artwork or Object Shown structure
- **THEN** its leaves appear as read-only rows and no write operation targets them

#### Scenario: An unsupported structure survives a supported edit

- **WHEN** a supported extension field is edited while an artwork structure is present
- **THEN** the artwork bytes are byte-identical to before

#### Scenario: Additional location structures are read-only

- **WHEN** the packet carries more than one Location Shown structure
- **THEN** the first is exposed as editable rows and the rest appear as read-only rows
