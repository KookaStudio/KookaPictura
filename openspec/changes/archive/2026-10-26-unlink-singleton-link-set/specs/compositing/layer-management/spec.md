## MODIFIED Requirements

### Requirement: Select Similar and Select Linked Layers

The system SHALL provide `Select Similar` and `Select Linked Layers`. `Select
Similar` SHALL select every layer that matches the active layer's kind and
attributes. `Select Linked Layers` SHALL select every member of the active
layer's link set. `Link Layers` SHALL add the selected layers to a link set,
joining existing sets when members already belong to one, and `Unlink Layers`
SHALL remove the selected layers from their sets and drop an emptied set. A link
set SHALL always have at least two members: when an unlink leaves a set with a
single member, that member SHALL be removed too, so a severed relationship shows
no link and `Select Linked Layers` no longer returns it. All four SHALL be
view/selection operations that do not change layer pixels and do not add a
history state. Link sets SHALL be transient session state: they SHALL NOT be
serialized, and a structural layer operation SHALL clear them (documented
limitation; the PSD link-set representation is unsourced).

#### Scenario: Select Similar matches the active layer

- **WHEN** Select Similar runs on a pixel layer
- **THEN** every layer of the same kind and matching attributes is selected and
  no pixels change

#### Scenario: Link and Select Linked

- **WHEN** two layers are linked and Select Linked Layers then runs on one of
  them
- **THEN** both members of the set are selected

#### Scenario: Unlink removes membership

- **WHEN** Unlink Layers runs on a linked layer
- **THEN** that layer is no longer in the set and linking/selecting no longer
  includes it

#### Scenario: Unlinking a member clears a two-layer set

- **WHEN** two layers are linked and Unlink Layers runs on one of them
- **THEN** neither layer is in a link set, both rows show no link indicator, and
  Select Linked Layers on either layer returns nothing

#### Scenario: Link sets are session-only

- **WHEN** a document with linked layers is saved and reopened, or a structural
  layer operation runs
- **THEN** the link sets are not persisted and are cleared by the structural
  operation
