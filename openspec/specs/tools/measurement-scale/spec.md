# Measurement Scale Specification

## Purpose

Calibrate document pixels to real-world units and reuse named calibrations across documents.

## Requirements

### Requirement: Document calibration

Each document SHALL have one effective measurement scale. Default SHALL be
`1 px = 1 px`. A custom scale SHALL have a whole Pixel Length in the positive
signed 32-bit range, a positive finite Logical Length, a positive finite
conversion factor, and a nonblank logical unit label.
Length SHALL convert by multiplying pixels by logical length / pixel length.
Area SHALL use the square of that ratio. Angles SHALL remain in degrees.

#### Scenario: A calibrated length and area [scale_conversion]

- **WHEN** the scale is 50 pixels to 1 micron
- **THEN** 100 pixels converts to 2 microns and 2500 square pixels converts to 1 square micron

#### Scenario: Invalid calibration is rejected [scale_invalid]

- **WHEN** a user enters a fractional or out-of-range Pixel Length, a zero, negative, or nonfinite length, or a blank unit label
- **THEN** the application refuses the change and keeps the active calibration

### Requirement: Calibration controls

Extended SHALL expose Default, Custom, and saved presets under Set Measurement
Scale. Custom SHALL activate the Ruler and let the user type Pixel Length or
populate it from a ruler drag rounded to the nearest whole pixel, with a
minimum of one for a nonzero drag. The dialog SHALL accept manual Logical
Length and Logical Units. Confirming a changed scale SHALL add one undoable
document state. Cancel and unchanged custom values SHALL add no history state.
Default SHALL reset the active scale and clear any unreadable scale resource.
The dialog SHALL cancel its draft when its opening document stops being active
or closes. The application SHALL use active document identity for this decision.
Switching between views of the same document or moving focus to the canvas
SHALL leave the draft open.
The submenu SHALL check Default for the default scale. For a custom scale it
SHALL check the matching named preset with the lexicographically smallest name,
or Custom when no preset matches. A match SHALL require equality of Pixel
Length, Logical Length, and Logical Units. Equivalent ratios alone SHALL NOT
match. Default and explicitly applied custom state SHALL remain distinct,
including custom units with an identity ratio. The check SHALL update on scale changes,
history restoration, preset changes, and active document changes.
The Info panel and document status SHALL offer a
measurement scale readout. The Ruler SHALL show calibrated length alongside
its existing pixel readout when a custom scale is active.

#### Scenario: Manual and ruler calibration entry [scale_dialog]

- **WHEN** a user opens Custom, supplies a pixel length by dragging the Ruler or typing, and confirms valid logical length and units
- **THEN** the active document uses that scale and the Ruler, Info panel, and document status can display it

#### Scenario: History and document isolation [scale_history]

- **WHEN** a user applies a different scale to one of two documents, selects Default, and uses Undo and Redo
- **THEN** each scale change creates one history state, Undo and Redo restore the corresponding values and readouts, and the other document retains its scale

#### Scenario: Cancel and repeated application [scale_noop]

- **WHEN** a user cancels a draft or accepts the already active values
- **THEN** the active values and document history are unchanged

#### Scenario: Standard calibration availability [scale_standard]

- **WHEN** the application runs in Standard
- **THEN** the measurement-scale actions and calibrated readouts are absent, and the regular Ruler remains available

#### Scenario: Draft document becomes inactive [scale_dialog_inactive]

- **WHEN** the opening document is switched away from or closed while Custom is open
- **THEN** the draft is canceled without changing any document or history

#### Scenario: The current scale is checked [scale_menu_check]

- **WHEN** scales, presets, history, or active documents change
- **THEN** Default, the lexicographically smallest matching preset, or Custom is checked according to the active scale

### Requirement: Calibration after image resizing

The shared successful document-resize path SHALL reset a recognized custom
scale on an actual uniform or nonuniform pixel resample. The reset SHALL set the
scale to Default within the resize history state and indicate to the user that
recalibration is required. Undo SHALL restore the previous scale and resource
edit intent. Resolution-only edits, same-dimension resizes, and failed resizes SHALL
preserve calibration.
A successful Perspective Crop that changes distances SHALL invalidate
calibration even when output dimensions are unchanged. Distance-preserving
translations and rotations and refused warps SHALL preserve calibration. If
invalidation occurs while the document is inactive, the indication SHALL be
delivered when it becomes active, unless Undo has restored calibration.
An implicit resample SHALL preserve opaque 1074 bytes in both editions. The
opaque calibration SHALL remain unavailable and the user SHALL be told that
recalibration is required.

#### Scenario: Resampling invalidates calibration [scale_resample]

- **WHEN** a calibrated document is uniformly or nonuniformly resampled
- **THEN** the scale resets with a recalibration indication, Undo restores it, and saving and reopening preserves the resulting state

#### Scenario: Resize preserves calibration where pixels are unchanged [scale_resize_preserve]

- **WHEN** resolution alone changes, a resize makes no pixel change, or resizing fails
- **THEN** the calibration remains unchanged

#### Scenario: Resampling preserves opaque scale data [scale_resample_opaque]

- **WHEN** a document with opaque 1074 is resampled in either edition
- **THEN** its bytes remain intact on save and the user is told recalibration is required

#### Scenario: Perspective Crop changes distances [scale_perspective]

- **WHEN** a Perspective Crop changes distances, or a distance-preserving or refused warp is attempted
- **THEN** only the successful distance-changing warp invalidates calibration and indicates recalibration, preserving opaque resource bytes

#### Scenario: Background invalidation is indicated [scale_background]

- **WHEN** a calibration is invalidated while its document is inactive
- **THEN** the indication appears when that document becomes active, unless Undo already restored calibration


### Requirement: Application scale presets

Extended SHALL let users save and delete named scale presets through Custom.
Presets SHALL persist across application restarts. Applying a preset SHALL copy
its values into the active document. Deleting a preset SHALL remove it from the
application menu and SHALL preserve every document’s active calibration.

#### Scenario: A preset is reused and deleted [scale_presets]

- **WHEN** a user saves a named preset, restarts, applies it to two documents, and deletes it
- **THEN** the preset was available after restart, disappears from the menu on deletion, and both documents keep the applied calibration
