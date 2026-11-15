# psd-xmp-metadata Specification

## Purpose
Parses, edits, and saves the XMP packet and synchronises the IPTC-Core fields with IIM.

## Requirements

### Requirement: The XMP packet is parsed into typed properties

`pictura-codec` SHALL expose a parser that reads the document's XMP image
resource (id 1060) into a fixed set of typed properties: title, creator,
description, subject, rights, credit, source, headline, marked, city, state,
country, instructions, authors position, caption writer, web statement, and
usage terms. The parser SHALL accept both the attribute form (`ns:prop="value"`)
and the element forms (`rdf:Alt` lang alternatives for title, description,
rights, and usage terms; `rdf:Seq`/`rdf:Bag` for creator/subject), SHALL NOT
resolve XML entities or fetch external resources, SHALL bound the work it does,
and SHALL NOT panic: a malformed or truncated packet SHALL yield the properties
decoded so far. A missing resource SHALL yield empty properties.

#### Scenario: The fixture's XMP yields its core properties

- **WHEN** the parser reads the metadata fixture's XMP packet
- **THEN** it returns the title, creator, description, rights, city, state, country, instructions, authors position, caption writer, web statement, and usage terms stored in the packet, and any property not present is empty

#### Scenario: An element-form and an attribute-form property both decode

- **WHEN** a packet stores one managed property as an `rdf:Alt` element and another as an attribute
- **THEN** both are decoded to their text value

#### Scenario: A truncated packet does not panic

- **WHEN** the packet is cut off inside an element
- **THEN** parsing returns the properties decoded so far and does not panic

### Requirement: XMP properties are edited by patching the packet in place

`pictura-codec` SHALL expose an operation that sets a managed property in the
document's XMP packet by replacing only the bytes of that property and copying
every other byte — unknown namespaces, unknown properties, comments, and the
packet wrapper — verbatim. Setting a property that is already its requested
value SHALL be a no-op returning false. A packet the writer does not recognise,
or that uses a construct it cannot safely rewrite, SHALL leave the resource
untouched and return false. Setting a property on a document with no XMP
resource SHALL frame a minimal well-formed packet. The operation SHALL preserve
every other image resource byte-for-byte and SHALL NOT drop an unparsed tail.

#### Scenario: Editing one property preserves an unknown namespace

- **WHEN** a property is set on a packet that also carries a property in an unknown namespace
- **THEN** the edited property has the new value and the unknown-namespace bytes are byte-identical to before

#### Scenario: An unrecognised packet is a no-op

- **WHEN** a property is set on a packet with no recognisable `rdf:Description`
- **THEN** the operation returns false and the image-resource section is unchanged

#### Scenario: Setting a property where none existed creates a packet

- **WHEN** a property is set on a document with no XMP resource
- **THEN** a well-formed XMP packet carrying the property is framed and the document's XMP parses back to that value

### Requirement: The IPTC-Core fields synchronise between XMP and IIM

`pictura-codec` SHALL expose an operation that applies a set of fourteen shared
fields — Object Name, By-line, By-line Title, Keywords, Copyright Notice,
Caption/Abstract, Credit, Source, Headline, Instructions, City, State/Province,
Country, and Writer/Editor — to **both** the XMP packet and the IIM record
(resource 1028) in one call, mapping each field to its XMP property. After the
operation the XMP property and the IIM record for a field SHALL hold the same
value, so the two channels do not conflict. Keywords SHALL be list-valued: each
value SHALL be one IIM dataset occurrence and one `dc:subject` bag item, and an
empty list SHALL remove every occurrence. The operation SHALL return whether
anything changed and SHALL be a no-op when every field already has its requested
value.

#### Scenario: An edited core field is written to both channels

- **WHEN** the Object Name is set through the synchronising operation
- **THEN** both the XMP title and the IIM Object Name hold the new value

#### Scenario: An Origin field is written to both channels

- **WHEN** City is set through the synchronising operation
- **THEN** both the XMP city property and the IIM City record hold the new value

#### Scenario: Keywords are written as a list

- **WHEN** Keywords is set to two values through the synchronising operation
- **THEN** the XMP subject bag carries both items and the IIM stream carries two Keywords datasets

#### Scenario: An unchanged field changes neither channel

- **WHEN** the operation is called with the values the document already holds
- **THEN** it returns false and both the XMP and IIM resources are unchanged

### Requirement: An edited document's XMP survives a save

The system SHALL commit a test that edits a fixture document's XMP properties,
writes it with `write_psd`, re-reads it, and asserts the edited values are
present and unknown properties survive; a second check SHALL confirm the
independent `exiftool` decoder reads the edited properties from the saved file,
self-skipping with a clear message when `exiftool` is unavailable. The engine's
parse and patch correctness SHALL also be proven by unit tests that do not
depend on `exiftool`.

#### Scenario: Edit, save, re-read keeps the XMP edit

- **WHEN** a fixture's title and credit are edited, saved, and read back
- **THEN** the parsed XMP title and credit are the edited values and an unknown property in the packet is unchanged

#### Scenario: exiftool agrees on the saved XMP edit

- **WHEN** `exiftool` decodes the saved file
- **THEN** its XMP title and credit match the edited values

#### Scenario: The oracle self-skips without exiftool

- **WHEN** the `exiftool` binary is not available
- **THEN** the oracle reports a skip and the suite passes

### Requirement: The raw XMP packet is replaceable

`pictura-codec` SHALL expose an operation that replaces the document's XMP image
resource (id 1060) with a caller-supplied packet only when the packet is
well-formed and within the decoder's packet-size bound. It SHALL preserve every
other image resource and any unparsed tail byte-for-byte, SHALL treat an empty
packet as removal of the resource, SHALL return whether anything changed, and
SHALL NOT mutate the document when the packet is rejected or unchanged.

#### Scenario: A valid packet replaces resource 1060

- **WHEN** a well-formed packet is supplied
- **THEN** reading the document returns that packet and every other image resource is byte-identical to before

#### Scenario: An ill-formed packet is rejected

- **WHEN** a packet with unbalanced or truncated markup is supplied
- **THEN** the operation returns false and the image-resource section is unchanged

#### Scenario: An empty packet removes the resource

- **WHEN** an empty packet is supplied
- **THEN** the document has no XMP resource and the other resources are unchanged
