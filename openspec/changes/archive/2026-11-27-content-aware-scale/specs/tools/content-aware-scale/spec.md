# Content-Aware Scale

## Purpose

Allow a user to resize raster content while preferentially preserving important image regions.

## ADDED Requirements

### Requirement: Content-Aware Scale session

The Edit menu SHALL provide Content-Aware Scale for one eligible raster layer or its selected pixels. Background, locked, adjustment, mask, channel, Smart Object, group, video and 3D targets, multiple layers, layers with attached raster or vector masks, and converted-mode layers without retained source channels SHALL be refused without changing the document. Selected-pixel operation SHALL be limited to 8-bit documents; whole-layer operation SHALL support 8/16/32-bit native samples and document color components. A session SHALL offer a scaling box, reference point, X/Y position, W/H percentages, relative positioning and aspect lock. It SHALL offer commit and cancel.

#### Scenario: Enter and resize [cas_session]
- **WHEN** Content-Aware Scale is invoked on an eligible raster layer and a handle or W/H field is changed
- **THEN** the session shows the resulting bounds and preview, with the reference point and aspect lock honored

#### Scenario: Ineligible target [cas_refusal]
- **WHEN** the command is invoked on an unsupported or locked target, a layer with a linked or unlinked mask, a converted-mode layer lacking retained source channels, or a selection in a 16/32-bit document
- **THEN** it is unavailable or refused and document pixels, masks, selection and history remain unchanged

#### Scenario: Selection limits the target [cas_selection]
- **WHEN** a nonempty selection intersects visible content on an eligible 8-bit layer and a scale is committed
- **THEN** coverage-weighted selected content is lifted, the complementary source coverage remains, and the result is composited normally over its destination, including unselected content there; the selection border is cleared

#### Scenario: Exclusive session [cas_exclusive]
- **WHEN** the session is pending and a document edit, competing edit session, tool switch or layer switch is requested
- **THEN** edits and competing sessions are refused while tool and layer switches use the existing Apply / Cancel / Don't Apply resolution, preserving the session if the switch is cancelled

### Requirement: Content-aware scaling and protection

The operation SHALL use deterministic forward-energy seam carving for shrinking and enlargement, operating on all component planes and alpha together. Amount SHALL range from 0 to 100 percent, default to 100, and blend content-aware scaling with ordinary scaling: 0 is ordinary scaling and 100 is content-aware scaling. Protect SHALL default to None and list document alpha channels by name. Channel coverage SHALL bias seam selection away from protected areas in document coordinates; the chosen channel SHALL remain unchanged. A protected area is preferentially, not absolutely, preserved. This is a behavioral approximation, not verified Adobe parity.

#### Scenario: Amount endpoints and intermediate [cas_amount]
- **WHEN** the same nonuniform fixture is resized with Amount 0, 50, and 100
- **THEN** 0 matches ordinary scaling, 100 uses seam carving, and 50 is the equal per-sample premultiplied blend of those outputs at the requested dimensions

#### Scenario: Protect alpha channel [cas_protect]
- **WHEN** a fixture with equally eligible seams is shrunk with an alpha channel protecting one candidate
- **THEN** the unprotected candidate is preferred, None applies no protection bias, and the channel remains unchanged

#### Scenario: Protect controls [cas_protect_ui]
- **WHEN** a session opens in a document with named alpha channels
- **THEN** Amount defaults to 100, Protect defaults to None and lists the named channels

#### Scenario: Both axes and enlargement [cas_axes]
- **WHEN** raster content is resized independently along either axis, including enlargement
- **THEN** the output has the requested nonzero dimensions, aligned color and alpha planes, and deterministic pixels

#### Scenario: Native components and depth [cas_native]
- **WHEN** whole-layer 16-bit and 32-bit non-RGB fixtures including out-of-range floating-point samples are resized
- **THEN** native depth and component alignment are retained and float samples are not clamped

#### Scenario: Invalid geometry [cas_invalid]
- **WHEN** the requested dimensions are zero, non-finite, unrepresentable, over the specified work or memory budget, or scratch allocation or admission is refused
- **THEN** the operation is refused without altering the document or history

### Requirement: Transactional preview and history

Preview SHALL derive from the original session source. Only the current completed preview SHALL be eligible to commit; superseded or cancelled work SHALL NOT publish or commit. Cancel or Escape SHALL restore the original document exactly. Commit SHALL create one history state, and undo and redo SHALL restore the exact pre- and post-commit document states.

#### Scenario: Superseded preview [cas_preview]
- **WHEN** a preview update is superseded by another update or the session is cancelled while work is pending
- **THEN** stale results cannot publish or commit, and commit is unavailable until the current result completes

#### Scenario: Cancel is lossless [cas_cancel]
- **WHEN** a preview is changed and then cancelled
- **THEN** the original pixels, geometry, selection and history are restored exactly

#### Scenario: Commit undo redo [cas_history]
- **WHEN** a modified session is committed, undone and redone
- **THEN** exactly one history state is added and undo and redo restore the exact original and committed states respectively
