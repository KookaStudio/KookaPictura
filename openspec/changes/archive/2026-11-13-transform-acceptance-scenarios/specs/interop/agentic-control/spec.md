## ADDED Requirements

### Requirement: A layer's pixels can be digested

`layer_digest` (Read tier) SHALL answer, for the layer at a `layer_tree` path,
a 64-bit digest of its rect, each channel's id and rows, and its mask's rect and
rows, together with the layer's rect. It SHALL read the rows tile by tile,
including tiles spilled to the scratch disk, and SHALL NOT build a full-size
image or change the document. An unknown path SHALL be `invalid_param`.

#### Scenario: Equal pixels digest equally and a change is seen [ce_layer_digest]

- **WHEN** a client digests a painted layer twice, and then again after one
  pixel of it changes
- **THEN** the first two digests are equal, the third differs, the rect is the
  layer's, and an unknown path is `invalid_param`

### Requirement: A control key reaches the canvas when no window is active

`key` SHALL send its key to the application's focus widget and, when there is
none (no window is active, as offscreen after a progress window closed), to the
main window's own focus child, before it falls back to the main window. A key
that a user would press in the window once it is activated SHALL reach the same
widget.

#### Scenario: Escape ends a transform while another window is active [ce_key_inactive_window]

- **WHEN** a Free Transform session is open, the canvas holds the main window's
  focus, another window is active, and a client sends `key` `Escape`
- **THEN** the canvas receives the key and the session ends
