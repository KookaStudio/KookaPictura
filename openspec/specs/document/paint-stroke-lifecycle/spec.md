# document/paint-stroke-lifecycle Specification

## Purpose
Define one begin and one finish operation for every painting-tool stroke, so a
layer stroke, a mask stroke, and every tool-specific stroke share the same slot,
present-frame, tile-set, label, recording, and commit-or-cancel contract.

## Requirements

### Requirement: Every painting-tool begin uses one stroke constructor

A paint stroke begin SHALL install its live session through one constructor. The
constructor SHALL own the single-session slot check, the history label, the paint
pad, the fade capture, the pending present, the stroke tile set, and tool
recording. A layer stroke and a mask stroke SHALL both enter through that
constructor, and the caller SHALL supply only the built stroke target and label
(plus the backend the caller has already chosen).

A refused begin SHALL install no session, clear no pending present, reset no
tiles, and open no recording.

#### Scenario: A layer stroke begins through the one constructor [osl_layer_begin]

- **WHEN** a Brush, Pencil, Eraser, or tool-specific painting begin starts on a
  visible pixel layer
- **THEN** exactly one paint session is installed with its label, paint pad,
  fade capture, empty pending present, reset tile set, and recording opened once

#### Scenario: A mask stroke begins through the same constructor [osl_mask_begin]

- **WHEN** a Brush or Eraser begins while a layer mask is the edit target
- **THEN** the mask stroke is installed through the same constructor, with the
  mask's paint pad and no fade capture, and no second begin path is used

#### Scenario: A refused begin leaves the live state untouched [osl_refused]

- **WHEN** a painting begin is refused because another session owns the slot or
  the target is not paintable
- **THEN** the live session, pending present, stroke tiles, and recording are
  unchanged and the begin returns false

### Requirement: Ending a stroke consumes one typed finish result

Ending a paint stroke SHALL take the live session and consume one typed finish
result instead of destructuring `StrokeSession`. The finish result SHALL name the
layer or mask target and carry the data that target needs to commit or cancel.
A layer finish SHALL choose between committing its recorded regions, restoring
its structural base, and recompositing a previewed stroke. A mask finish SHALL
replay a previewed stroke, write the mask back, and choose between committing
its regions and recompositing.

#### Scenario: A layer stroke commits one history state [osl_layer_commit]

- **WHEN** a layer stroke with dirty pixels ends
- **THEN** the typed layer finish result records exactly one history state with
  the stroke's label and its tile-derived regions, and the committed pixels are
  unchanged from the existing stroke path

#### Scenario: A mask stroke commits one history state [osl_mask_commit]

- **WHEN** a mask stroke with dirty coverage ends
- **THEN** the typed mask finish result writes the coverage back, records exactly
  one history state with the mask's label and regions, and leaves the composite
  rule unchanged

#### Scenario: An empty stroke commits nothing [osl_empty]

- **WHEN** a layer or mask stroke ends with no changed pixels
- **THEN** the finish result records no history state and restores any structural
  base or preview state exactly as before

#### Scenario: A previewed stroke replays exactly [osl_preview_replay]

- **WHEN** a large-brush layer or mask stroke that presented at a reduced level
  ends
- **THEN** the finish result replays its samples through the exact stroke before
  committing, so the committed pixels equal the full-resolution stroke
