## MODIFIED Requirements

### Requirement: Free Transform over layer_op

`layer_op` SHALL accept op `free_transform` with `index`, `scale` (percent,
both axes, default 100) and `angle` (degrees, default 0). It SHALL also accept
an optional `warp` (a warp preset id, such as `arc`) and `bend` (percent,
default 50). It SHALL begin a Free Transform session on the layer through the
transform tool, and set the width, height and angle as the options bar does.
When `warp` is given, it SHALL enter Warp, pick that preset and set its bend, as
the Warp options bar does. It SHALL then commit the session, so a large layer's
commit defers as it would for a person. An out-of-range `index`, a `scale`
outside (0, 10000], a non-finite `angle`, or a `bend` that is not finite or lies
outside [−100, 100] SHALL return `invalid_param`. The op SHALL return `refused`,
and leave any open session as it was, while a transform session is already
open, when the layer cannot be transformed, when the warp preset is unknown, or
when the commit does not happen.

#### Scenario: A layer is scaled and rotated [mb_free_transform]

- **WHEN** a client invokes `layer_op` with op `free_transform`, a layer's
  index, `scale` 150 and `angle` 15
- **THEN** after the commit settles the layer's bounds are larger than before
  and the history has one more state

#### Scenario: An open session is not taken over [mb_free_transform_busy]

- **WHEN** a Free Transform session is open and a client invokes
  `free_transform`
- **THEN** the response is `refused` and the session is still open

#### Scenario: A layer is warped [mb_free_transform_warp]

- **WHEN** a client invokes `layer_op` with op `free_transform`, a layer's
  index, `warp` `arc` and `bend` 50
- **THEN** after the commit settles the layer's top edge has moved up, the
  history has one more state, and no transform session is open; a `bend` of 150
  returns `invalid_param` and an unknown `warp` returns `refused`
