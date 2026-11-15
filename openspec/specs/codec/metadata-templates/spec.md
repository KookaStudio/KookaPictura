# metadata-templates Specification

## Purpose
Serializes managed XMP to a standalone template and applies it with merge modes and IPTC synchronisation.

## Requirements

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

### Requirement: A document exports its metadata as a standalone template

`pictura-codec` SHALL expose an export that returns the document's managed XMP
properties serialized as a standalone packet (the template bytes a caller writes
to a `.xmp` file). The export SHALL NOT include the document's EXIF, ICC, or any
unmanaged XMP property, so a template carries only the managed fields.

#### Scenario: Export carries the managed fields only

- **WHEN** a document whose XMP carries a title and an unknown-namespace property is exported
- **THEN** the exported bytes parse to a title-equal property set and contain no unknown-namespace property

### Requirement: A template applies with a merge mode

`pictura-codec` SHALL expose `apply_template(&mut Document, &XmpProperties,
MergeMode) -> bool` with modes Append, Replace, and KeepOriginalReplaceMatching:
Append SHALL set a managed field only when the document's current value for it is
empty or absent; Replace SHALL set every managed field from the template and
clear the fields the template omits; KeepOriginalReplaceMatching SHALL overwrite
only the fields the template defines and SHALL clear nothing. Every mode SHALL
patch the document's existing packet in place, so unknown namespaces, unknown
properties, EXIF, and every other image resource survive byte-for-byte. The
function SHALL return whether anything changed and SHALL be a no-op returning
false when the mode would change no field.

#### Scenario: Append fills only empty fields

- **WHEN** Append is applied to a document with an existing title and an empty credit, and the template sets both
- **THEN** the title is unchanged and the credit becomes the template value

#### Scenario: Replace overwrites and clears omitted fields

- **WHEN** Replace is applied with a template that sets the title and omits the credit
- **THEN** the title is the template value and the credit is cleared

#### Scenario: KeepOriginalReplaceMatching overwrites only template fields

- **WHEN** KeepOriginalReplaceMatching is applied with a template that sets the title and omits the credit
- **THEN** the title is the template value and the credit is unchanged

#### Scenario: Unknown properties and camera data survive an apply

- **WHEN** a template is applied to a document whose XMP carries an unknown-namespace property and whose resources carry EXIF
- **THEN** the unknown-namespace bytes and the EXIF resource are byte-identical to before

#### Scenario: Applying an empty template in Replace mode clears nothing unexpected

- **WHEN** an empty template is applied in Append or KeepOriginalReplaceMatching mode
- **THEN** the function returns false and no managed field changes

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

### Requirement: A template apply is a single undoable step and survives a save

The application SHALL expose Export (write the template to a chosen `.xmp` path)
and Apply (choose an `.xmp` path and a merge mode) from File Info. Applying a
template SHALL record exactly one undo state labelled for the edit and mark the
document dirty; undoing SHALL restore the pre-apply metadata. Export SHALL NOT
mutate the document or record history. The system SHALL commit a test that
applies a template, writes the document, re-reads it, and asserts the applied
values persist and unknown properties and camera data survive; a second check
SHALL confirm the independent `exiftool` decoder reads the applied fields,
self-skipping when `exiftool` is unavailable. Engine unit tests SHALL prove the
three merge modes without `exiftool`.

#### Scenario: Apply records one state and undo restores

- **WHEN** a template is applied and then undone
- **THEN** exactly one history state was added and the document's metadata equals its pre-apply state

#### Scenario: Export does not touch the document

- **WHEN** a template is exported
- **THEN** the document's image resources are unchanged and no history state is added

#### Scenario: Apply, save, re-read keeps the template values

- **WHEN** a template is applied, the document is written with `write_psd`, and read back
- **THEN** the applied managed fields are present and the unknown-namespace property and EXIF resource survive

#### Scenario: exiftool agrees on the applied template

- **WHEN** `exiftool` decodes the saved document
- **THEN** its XMP title and credit match the applied template

#### Scenario: The oracle self-skips without exiftool

- **WHEN** the `exiftool` binary is not available
- **THEN** the oracle reports a skip and the suite passes
