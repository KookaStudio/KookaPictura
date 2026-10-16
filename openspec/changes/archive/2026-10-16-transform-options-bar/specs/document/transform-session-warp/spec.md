# transform-session-warp Specification

## Purpose

The in-session preset warp: a warp chosen during a Free Transform session is
part of the session state, previewed live, and committed with the transform as
one history state.

## ADDED Requirements

### Requirement: In-session warp state and dialog

The Free Transform session SHALL hold an optional preset warp carrying a warp
style id, a bend, X/Y distortion percents, and a vertical-orientation flag. The
application SHALL expose a warp button on the transform options bar that opens
the existing `WarpPresetDialog`; on accept, a style with a preset mesh SHALL be
stored on the session (the mesh built for the current target rect) and
`None`/`Custom` (no preset mesh) SHALL clear the warp. The session SHALL report
whether a warp is set and its values through the bridge, and SHALL accept a
direct set and a clear.

#### Scenario: A preset becomes session state

- **WHEN** the warp dialog is accepted with the Arc style at 50% bend during a session
- **THEN** the session reports an active warp with the Arc id and 50% bend

#### Scenario: None clears the warp

- **WHEN** the warp dialog is accepted with the None style while a warp is set
- **THEN** the session reports no active warp

#### Scenario: No session, no warp

- **WHEN** a warp is set with no active session
- **THEN** the call returns false and no state changes

### Requirement: Warped preview and suspended gestures

While a warp is set the session SHALL show a preview of the warped layer,
computed on a document clone by applying the live transform and then the warp
mesh, without mutating the document; the preview SHALL use the session's
interpolation. Box gestures (scale, rotate, move, distort, skew, perspective)
SHALL be suspended while a warp is set: a press away from the reference point
SHALL report no hit. The document MUST equal its pre-session value until
commit.

#### Scenario: The preview mutates nothing

- **WHEN** a warp is set and the preview is computed
- **THEN** the document equals its pre-session value and no history state is added

#### Scenario: Box gestures are suspended while warped

- **WHEN** a warp is set and a press lands on a box handle
- **THEN** no gesture begins

### Requirement: Warp commit contract

Committing a warped session SHALL apply the live similarity or projective
transform first and the warp mesh second, resampling with the session's
interpolation, recomposite, and record exactly one `"Free Transform"` history
state when anything changed. The warp mesh for the commit SHALL be built from
the target rect after the transform. An identity session (no transform, no
warp) SHALL record nothing. Escape SHALL end the session with the document
bit-identical to its pre-session value and no history state added.

#### Scenario: Transform then warp records one state

- **WHEN** a session scales its layer and sets a warp, then commits
- **THEN** the layer is scaled and warped and exactly one `"Free Transform"` state is added

#### Scenario: A warp-only session commits the warp

- **WHEN** a session sets a warp without any box gesture and commits
- **THEN** the layer is warped and exactly one `"Free Transform"` state is added

#### Scenario: Escape restores a warped session exactly

- **WHEN** a session sets a warp and Escape is pressed
- **THEN** the document equals its pre-session value and the history count is unchanged
