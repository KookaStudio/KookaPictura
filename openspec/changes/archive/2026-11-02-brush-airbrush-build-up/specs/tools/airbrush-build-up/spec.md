## ADDED Requirements

### Requirement: Brush Airbrush build-up

The Brush options bar SHALL carry an Airbrush toggle, off by default, and the
Brush panel's Build-up checkbox SHALL show the same flag. While a Brush stroke is
open with Airbrush on, the tool SHALL keep laying dabs at the pointer at a
bounded time-driven rate even when the pointer is still, accumulating toward the
Opacity cap. With Airbrush off, a still pointer SHALL add no paint. Held dabs
SHALL stop when the stroke ends, whether by release, tool switch, or session
cancel, and SHALL NOT add a second history state.

#### Scenario: Airbrush off adds nothing while held [brush_airbrush_off]

- **WHEN** a Brush stroke with Airbrush off is held still at low Flow
- **THEN** the pixels under the pointer do not change

#### Scenario: Airbrush on builds while held [brush_airbrush_on]

- **WHEN** a Brush stroke with Airbrush on is held still at low Flow and then
  released
- **THEN** the pixels under the pointer move toward the foreground colour and
  release records one history state

#### Scenario: The options bar and Brush panel are one flag [brush_airbrush_two_views]

- **WHEN** the Brush options-bar Airbrush toggle is clicked, or the Brush panel's
  Build-up checkbox is changed
- **THEN** the model flag and the other view show the same state

#### Scenario: Held dabs stop when the stroke ends [brush_hold_stops]

- **WHEN** a held Brush stroke with Airbrush on is released, or a tool switch or
  session cancel ends it
- **THEN** the pixels stop changing while the event loop keeps running

### Requirement: Airbrush toggle shortcut

`Shift+Alt+P` SHALL toggle the active tool's Airbrush option. The Brush, Clone
Stamp, Pattern Stamp, and History Brush each have one; a tool without an Airbrush
option SHALL ignore the key. The options bar and the Brush panel SHALL follow the
model when the key changes it.

#### Scenario: The shortcut toggles the Brush [brush_shortcut]

- **WHEN** `Shift+Alt+P` is pressed with the Brush active
- **THEN** the Brush's Airbrush flag flips, the options-bar toggle follows, and
  the Brush panel's Build-up row follows

#### Scenario: The shortcut toggles a stamp's own option [stamp_shortcut]

- **WHEN** `Shift+Alt+P` is pressed with Clone Stamp, Pattern Stamp, or History
  Brush active
- **THEN** that tool's own Airbrush flag flips and its options-bar toggle follows,
  leaving the other tools' flags unchanged

#### Scenario: A tool without an Airbrush option ignores the key [brush_shortcut_ignored]

- **WHEN** `Shift+Alt+P` is pressed with a tool that has no Airbrush option
  active
- **THEN** no Airbrush flag changes

### Requirement: Held dabs on every Brush stroke backend

A held Brush stroke SHALL keep building on every backend the Brush paints
through: the exact CPU stroke, the reduced-level preview, the GPU stroke, and a
layer-mask stroke. A previewed stroke's commit SHALL replay its held dabs, so the
committed pixels include the build-up.

#### Scenario: Preview build-up reaches the commit [brush_preview_hold]

- **WHEN** a large-brush stroke previews at a reduced level with Airbrush on, is
  held, and is committed
- **THEN** the committed pixels are darker than the same stroke with the held
  dabs omitted

#### Scenario: Mask build-up reaches the commit [brush_mask_hold]

- **WHEN** a layer-mask Brush stroke with Airbrush on is held and committed,
  exactly and through the preview
- **THEN** the mask coverage includes the held build-up and the commit records
  one history state

#### Scenario: GPU build-up [brush_gpu_hold]

- **WHEN** a GPU Brush stroke with Airbrush on is held and committed
- **THEN** the committed pixels include the held build-up
