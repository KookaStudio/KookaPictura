# Spec Delta

## MODIFIED Requirements

### Requirement: Space temporarily activates Hand panning

Holding the `Space` key over the canvas SHALL temporarily enable Hand panning
while the previous tool remains logically active: the cursor SHALL become
`Qt::OpenHand`, and pressing and dragging SHALL pan the canvas through the shared
range helper exactly as the Hand tool does. Releasing `Space` SHALL restore the
previous tool's cursor and pan state, so a Space pan does not change the active
tool, the tool's options, or any document state. The `Space` key SHALL NOT be
bound as a prefix of a key sequence, so it is delivered as a plain key and never
consumed waiting for a second key. While `Space` is held, this override SHALL
take precedence over every other cursor and pan update — including pointer
hover, a tool switch, and any document or tool-state refresh — so the hand
cursor and the pan-enabled state persist for the whole hold and no later update
reverts the cursor or disables panning. A cursor-mode update that lands mid-hold
SHALL NOT drop a subsequent pan or let the press reach the active tool. The hold
SHALL end only when the key is physically released: a key auto-repeat event SHALL
NOT be treated as a release, so holding `Space` keeps the override active.

#### Scenario: Space pans without changing the tool [lct_space_pan]

- **WHEN** `Space` is held and the canvas is dragged with a non-Hand tool active
- **THEN** the canvas pans, the active tool is unchanged, and the offset is
  clamped by the shared range

#### Scenario: Releasing Space restores the cursor [lct_space_release]

- **WHEN** `Space` is released after a pan
- **THEN** the previous tool's cursor and pan state are restored

#### Scenario: Space is not a sequence prefix [lct_space_not_prefix]

- **WHEN** `Space` is pressed and released on its own
- **THEN** it is handled as the temporary pan key and is not held waiting for a
  second key in a sequence

#### Scenario: The override survives a cursor refresh [lct_space_refresh]

- **WHEN** a cursor-mode update occurs while `Space` is held — a pointer hover,
  a tool switch, or a document or tool-state refresh
- **THEN** the cursor remains `Qt::OpenHand` and panning remains enabled, so the
  hand cursor does not flash and revert

#### Scenario: Hover does not clobber the hand cursor [lct_space_hover]

- **WHEN** the pointer moves over the canvas with `Space` held and no mouse
  button down
- **THEN** the cursor stays `Qt::OpenHand` and no tool hover or cursor state is
  applied

#### Scenario: A mid-hold update does not drop the pan [lct_space_midhold]

- **WHEN** a cursor-mode update lands while `Space` is held and the left button
  is then pressed and dragged
- **THEN** the canvas pans and no tool edit or drag begins, regardless of the
  tool that was active before the press

#### Scenario: Key auto-repeat does not end the override [lct_space_autorepeat]

- **WHEN** `Space` is held and a key auto-repeat event (a synthetic release that
  is not a physical release) arrives
- **THEN** the hand cursor and pan-enabled state persist and a subsequent drag
  still pans

#### Scenario: A transform session ends the override [lct_space_transform]

- **WHEN** a Free Transform session starts while `Space` is held
- **THEN** the transient hand override ends and the transform session's
  pan-disabled, modal state takes over

#### Scenario: A lost key-up releases the override [lct_space_focus]

- **WHEN** the canvas loses focus or the window deactivates while `Space` is held
- **THEN** the override ends so a missed key-up cannot leave every later press
  routed to panning
