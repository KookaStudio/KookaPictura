# Spec Delta

## ADDED Requirements

### Requirement: Free Transform distort and skew shortcuts

In a `Free` session, a corner press with Ctrl (and not the Ctrl+Alt+Shift
perspective combination) SHALL start a distort drag: the drag SHALL move that
corner by the press-to-pointer delta, keeping the grab offset, stopping where
the quad would become concave, and carrying the reference point as a `Distort`
corner drag does. A side-handle press with Ctrl+Shift SHALL start a skew drag
that follows the `Skew` edge rules: an edge handle SHALL slide its two
endpoints only along the edge's own axis, and Alt SHALL mirror the opposite
edge. A corner press with Ctrl+Shift SHALL remain a distort drag because the
skew shortcut is defined on side handles only. Ctrl on a side handle without
Shift SHALL keep the ordinary scale drag.

The session SHALL store the edit as its source-space projective base and
apply the existing similarity after it, so later plain drags scale, rotate, and
move the reshaped box and further shortcut drags edit the same base. A session
with a base and no stored warp SHALL preview and commit through the projective
quad op and record exactly one `"Free Transform"` state; an unchanged shortcut
drag SHALL record none; Escape SHALL leave the document bit-identical. The
distort shortcut SHALL be ignored, leaving an ordinary scale drag, on an
editable type layer unless a lifted selection targets its pixels; the skew
shortcut SHALL keep an editable type layer a parallelogram, as the `Skew` mode
does. A press on the reference point, inside the box, or on the rotate band
SHALL keep its ordinary behavior with the modifiers held. The gesture
composition is inferred from the CS6 modifier table (no Adobe oracle).

#### Scenario: Ctrl on a corner distorts it [distort_shortcut]

- **WHEN** a `Free` session presses a corner with Ctrl and drags it by a delta with both a horizontal and a vertical component
- **THEN** that corner lands at its press-time position plus the delta, the other three corners stay put, and the box's opposite edges are no longer parallel

#### Scenario: A shortcut drag commits through the quad op [distort_shortcut_commit]

- **WHEN** a Ctrl corner drag is released and the session commits
- **THEN** the layer's corner lands on the dragged position and exactly one `"Free Transform"` state is added

#### Scenario: Ctrl+Shift on a side handle skews its edge [skew_shortcut]

- **WHEN** a `Free` session presses a side handle with Ctrl+Shift and drags it diagonally
- **THEN** the edge's two endpoints move only along the edge's axis and the opposite edge stays put

#### Scenario: A plain drag scales the shortcut box [distort_shortcut_scale]

- **WHEN** a `Free` session with a base from a Ctrl corner drag drags an edge handle without modifiers
- **THEN** the opposite edge stays fixed and the box keeps its reshaped, non-parallel edges

#### Scenario: The distort shortcut is ignored on editable type [distort_shortcut_type]

- **WHEN** a `Free` session on an editable type layer presses a corner with Ctrl and drags
- **THEN** the drag scales the box along the ordinary scale rules and the box keeps its parallel edges

#### Scenario: The skew shortcut keeps editable type a parallelogram [skew_shortcut_type]

- **WHEN** a `Free` session on an editable type layer presses a side handle with Ctrl+Shift and drags
- **THEN** the quad stays a parallelogram, the committed layer stays type, and exactly one `"Free Transform"` state is added
