## ADDED Requirements

### Requirement: Convert Point tool

Dragging from a Work Path anchor SHALL pull out a mirrored pair of handles,
making it smooth; clicking an anchor SHALL make it a corner with no handles;
dragging a handle SHALL move only that handle, leaving the point no longer
smooth. Each click or drag that changes the point SHALL record exactly one
"Convert Point" state; a click on an anchor that is already a corner SHALL
record nothing.

#### Scenario: Corner to smooth and back

- **WHEN** the `tst_pen_tools` test drags a square's corner, drags one of its new handles, and clicks the anchor twice
- **THEN** three "Convert Point" states are recorded: the point turns smooth with mirrored handles, then keeps its other handle when one is dragged, then becomes a corner, and the second click records nothing
