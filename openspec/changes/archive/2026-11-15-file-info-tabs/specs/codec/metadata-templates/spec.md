# Spec Delta

## MODIFIED Requirements

### Requirement: The managed XMP properties serialize to a standalone packet

`pictura-codec` SHALL expose a serializer that turns the fixed managed property
set (title, creator, description, subject, rights, credit, source, headline,
marked, city, state, country, instructions, authors position, caption writer,
web statement, and usage terms) into a standalone, well-formed XMP packet,
writing each property in its RDF form (`rdf:Alt` for title/description/rights/
usage terms, `rdf:Seq` for creator, `rdf:Bag` for subject, a simple element for
the `photoshop:` properties and for `xmpRights:Marked`/`xmpRights:WebStatement`).
Parsing the
serialized packet SHALL recover the same properties. An empty property set SHALL
serialize to a packet that parses back to empty properties.

#### Scenario: Serialize then parse round-trips

- **WHEN** a property set with a title, a two-item creator, a marked value, and an Origin field is serialized and parsed back
- **THEN** the recovered properties equal the original

#### Scenario: An empty set round-trips

- **WHEN** an empty property set is serialized and parsed back
- **THEN** the recovered properties are empty

### Requirement: A template apply synchronises the IPTC-Core fields to IIM

Applying a template SHALL write the fourteen shared fields (Object Name,
By-line, By-line Title, Keywords, Copyright Notice, Caption/Abstract, Credit,
Source, Headline, Instructions, City, State/Province, Country, and Writer/Editor)
to the IIM record (resource 1028) as well as the XMP packet, so the two channels
hold the same value afterwards; the remaining managed properties are written to
XMP only. Keywords SHALL be written as one IIM dataset occurrence per value and
one `dc:subject` bag item per value.

#### Scenario: An applied core field reaches both channels

- **WHEN** a template setting the title is applied
- **THEN** the parsed XMP title and the IIM Object Name are both the template value

#### Scenario: An applied Origin field reaches both channels

- **WHEN** a template setting the City is applied
- **THEN** the parsed XMP city and the IIM City record are both the template value

#### Scenario: An applied keyword list reaches both channels

- **WHEN** a template setting two keywords is applied
- **THEN** the XMP subject bag carries both values and the IIM stream carries two Keywords datasets
