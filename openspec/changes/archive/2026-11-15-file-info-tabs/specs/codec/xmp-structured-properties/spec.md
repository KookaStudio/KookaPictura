# Spec Delta

## Purpose

Reads an XMP packet as a generic property tree and patches simple, lang-alternative, bag, and one-level struct properties by path while preserving every unknown byte.

## ADDED Requirements

### Requirement: The XMP packet is readable as a generic property tree

`pictura-codec` SHALL expose a reader that returns every property in a requested namespace as a path, a textual value, and the property's form: an attribute, a simple element, a lang alternative, or a bag/sequence item. It SHALL resolve namespace prefixes from the declarations in scope, SHALL support both the attribute and element forms, SHALL bound the work it does, and SHALL NOT panic: a malformed or truncated packet SHALL yield the properties decoded so far. A packet with no recognisable `rdf:Description` SHALL yield no properties.

#### Scenario: Attribute-form and element-form properties both read

- **WHEN** a packet stores one property as an attribute and another as a child element
- **THEN** both are returned with the same namespace and their respective values

#### Scenario: Prefixes are resolved from the declarations in scope

- **WHEN** the same namespace is bound to different prefixes on different elements
- **THEN** each property is reported under its namespace URI, not its prefix

#### Scenario: A bag and a struct read as separate leaves

- **WHEN** a packet carries a bag of text items and a struct of child properties
- **THEN** the bag is reported as one value per item and the struct as one value per leaf path

#### Scenario: A malformed XMP packet does not panic

- **WHEN** the packet is truncated or a tag is unbalanced
- **THEN** the reader returns the properties decoded so far and does not panic

### Requirement: Properties are patched by path while preserving unknown bytes

`pictura-codec` SHALL expose an operation that sets or removes a property at a path by replacing only that property's bytes and copying every other byte — unknown namespaces, unknown properties, comments, and the packet wrapper — verbatim. It SHALL support creating a missing simple element, lang alternative, bag, and one-level struct; when the inserted property's namespace is not declared on the receiving description, the insertion SHALL declare it on the inserted element. Setting a property to its current value SHALL be a no-op. An unsupported property shape SHALL leave the packet unchanged and report failure rather than rewriting it. Every other image resource and any unparsed tail SHALL be preserved byte-for-byte.

#### Scenario: Patching a leaf preserves an unknown namespace

- **WHEN** a leaf is patched in a packet that also carries a property in an unknown namespace
- **THEN** the leaf holds the new value and the unknown-namespace bytes are byte-identical to before

#### Scenario: A missing bag is created

- **WHEN** a bag property is set on a packet that does not carry it
- **THEN** a well-formed bag with one item per requested value is inserted and reads back with those values

#### Scenario: A missing struct is created with a local namespace declaration

- **WHEN** a struct leaf is set on a packet whose description does not declare the struct's namespace
- **THEN** the inserted element declares the namespace locally and the leaf reads back with the requested value

#### Scenario: An unsupported shape is left unchanged

- **WHEN** the property uses a construct the patcher cannot safely rewrite
- **THEN** the operation reports failure and the image-resource section is byte-identical to before

#### Scenario: Removing a property removes only its bytes

- **WHEN** a property is removed
- **THEN** the property is absent when the packet is read again and the surrounding bytes are unchanged
