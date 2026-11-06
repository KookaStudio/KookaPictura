## ADDED Requirements

### Requirement: Free Transform over layer_op

`layer_op` SHALL accept op `free_transform` with `index`, `scale` (percent,
both axes, default 100) and `angle` (degrees, default 0). It SHALL begin a Free
Transform session on the layer through the transform tool, set the width,
height and angle as the options bar does, and commit it, so a large layer's
commit defers as it would for a person. An out-of-range `index`, a `scale`
outside (0, 10000] or a non-finite `angle` SHALL return `invalid_param`. While
a transform session is already open, or when the layer cannot be transformed
or the commit does not happen, the op SHALL return `refused` and leave any
open session as it was.

#### Scenario: A layer is scaled and rotated [mb_free_transform]

- **WHEN** a client invokes `layer_op` with op `free_transform`, a layer's
  index, `scale` 150 and `angle` 15
- **THEN** after the commit settles the layer's bounds are larger than before
  and the history has one more state

#### Scenario: An open session is not taken over [mb_free_transform_busy]

- **WHEN** a Free Transform session is open and a client invokes
  `free_transform`
- **THEN** the response is `refused` and the session is still open

### Requirement: A new document with a Background layer

The control server's `document` `new` SHALL accept `background_layer`
(default false). When true, the document SHALL be built as File ▸ New builds
it: a white document whose single layer is the locked `Background` without an
alpha channel, with one history state. `background_layer` with a background
other than `white` SHALL return `invalid_param`. Without it, `new` SHALL
behave as before.

#### Scenario: The new document's layer is the Background [mb_new_background]

- **WHEN** a client invokes `document` `new` with `background_layer` true
- **THEN** the active document has one layer named `Background`, white pixels
  and one history state, and the same request with a transparent background
  is refused with `invalid_param`
