# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
