# pen-edit-session Specification

## Purpose

The Pen-group tools' live gesture is a transient, cancellable edit step:
Edit > Undo cancels it before it reaches the document history, so an errant
anchor or handle drag never needs a release first.

## ADDED Requirements

### Requirement: Pen live gesture cancellation

While a Pen-group tool holds a live gesture — placing an anchor and dragging
its handles, dragging an existing handle with Convert Point, or drawing a
Freeform Pen trail — the system SHALL treat that gesture as one modal tool
session step. Edit > Undo SHALL cancel the gesture and restore the path to its
state before the press, without touching the document history; a second
Edit > Undo SHALL then step back the last committed pen edit. Releasing the
pointer after a cancelled gesture SHALL record no history state. Every
committed pen edit SHALL keep recording exactly one document history state.

#### Scenario: Undo cancels a live anchor drag [pes_cancel_drag]

- **WHEN** the `tst_pen_tools` test presses for a new anchor, drags its handles
  without releasing, and invokes Edit Undo
- **THEN** the live anchor is gone, the previously committed anchor is intact,
  and the document history index is unchanged

#### Scenario: Releasing after a cancelled gesture records nothing [pes_release_no_state]

- **WHEN** the pointer is released after Edit Undo cancelled the live gesture
- **THEN** no history state is recorded and the path equals its pre-press state

#### Scenario: Undo cancels the first anchor [pes_cancel_first]

- **WHEN** the test presses for the first anchor of a path without releasing and
  invokes Edit Undo
- **THEN** the path is empty and no history state exists

#### Scenario: A committed edit still steps back one state [pes_committed_walk]

- **WHEN** the test places three anchors, releasing each, and invokes Edit Undo
  once per anchor
- **THEN** each invocation removes exactly one anchor and records no other state

#### Scenario: A freeform trail is cancelled [pes_cancel_freeform]

- **WHEN** the test drags a Freeform Pen trail without releasing and invokes
  Edit Undo
- **THEN** the trail preview is cleared, the path is unchanged, and releasing
  records no state
