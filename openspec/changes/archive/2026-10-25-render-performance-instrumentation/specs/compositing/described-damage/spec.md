# Spec Delta

## ADDED Requirements

### Requirement: Content-neutral layer creation does not recomposite

Clipped layers attach to the nearest unclipped sibling below them. Inserting a
new empty pixel layer or a new empty group into a container is content-neutral
only when all of the following hold:

- the document had at least one layer before the insertion;
- the new node carries no pixels, no mode-neutral fill, no mask, and no effects;
- the sibling directly above the new node, if any, is not clipped, so no
  existing clipped run is moved to a new base;
- the new node is unclipped, or it is clipped and the sibling directly below it
  is itself clipped, so it joins an existing non-empty clipped run and no base
  gains its first clipped layer.

A content-neutral insertion SHALL NOT run the compositor and SHALL NOT mark
display damage. With CPU compositing, the stored document composite SHALL remain
byte-identical to `composite_rgba` of the new layer tree. Every other insertion
SHALL keep the full recomposite. So SHALL a content-neutral insertion made while
a live gesture (a stroke, a preview, a placement) is open, because the stored
composite is stale until that gesture ends.

This applies to New Layer and New Group from the Layers panel, from the
`Layer > New` dialog, from the `_in` path variants, and from the control server.

#### Scenario: New Layer runs no composite

- **WHEN** a new empty layer is created above an unclipped pixel layer whose
  upper sibling is unclipped
- **THEN** the compositor does not run, and the stored composite equals
  `composite_rgba` of the new document byte for byte

#### Scenario: New Group runs no composite

- **WHEN** a new empty group is created under the same conditions
- **THEN** the compositor does not run and the stored composite equals a full
  recomposite

#### Scenario: Insertion under a clipped sibling recomposites

- **WHEN** a new unclipped layer is inserted directly below a clipped layer
- **THEN** the insertion is not content-neutral and the full recomposite runs

#### Scenario: A first clipped layer recomposites

- **WHEN** the New Layer dialog creates a clipped layer directly above a base
  with no clipped layers yet
- **THEN** the insertion is not content-neutral and the full recomposite runs

#### Scenario: Joining an existing clipped run is neutral

- **WHEN** a clipped empty layer is inserted directly above a clipped layer
- **THEN** the insertion is content-neutral

#### Scenario: The first layer of an empty document recomposites

- **WHEN** a new layer is created in a document with no layers
- **THEN** the full recomposite runs

#### Scenario: Mode-neutral fill recomposites

- **WHEN** the New Layer dialog creates an Overlay layer with Fill With
  Mode-Neutral Color on
- **THEN** the insertion is not content-neutral
