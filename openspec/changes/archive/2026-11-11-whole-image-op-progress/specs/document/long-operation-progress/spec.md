## ADDED Requirements

### Requirement: Whole-document commands run as cancellable jobs

An interactive Image Size, Canvas Size, Rotate, Flip, mode conversion, bit-depth
or HDR conversion, Merge Down, Merge Layers, Merge Visible, Merge Clipping Mask,
or Flatten Image on a document of 4 MP or more SHALL run its engine on a worker
as a cancellable job, while the document and history stay unchanged until the
job lands. A completed job SHALL record the same single history state as the
synchronous command. A cancelled job SHALL leave the document pixel-identical
and the history count and index unchanged. A command on a smaller document, or
one not issued from the menus or panels, SHALL apply at once.

#### Scenario: A large rotation runs as a job [lop_whole_image_job]

- **WHEN** Image > Image Rotation > 90° CW runs on a 2400 × 1800 document whose job is held by the test seam
- **THEN** the progress window appears, and on landing the document is 1800 × 2400 with one added `"Rotate"` history state

#### Scenario: A cancelled whole-document command changes nothing [lop_whole_image_cancel]

- **WHEN** Flatten Image runs on a large two-layer document and Cancel is pressed while its job is held
- **THEN** the layer count, the history count and index, and the displayed pixels are unchanged, and the command applied afterwards adds one state

#### Scenario: A merge job reports the merged layer [lop_merge_job]

- **WHEN** a Merge Down job lands
- **THEN** the merged layer is the active target and the reported outcome names its path and the two layers it replaced

#### Scenario: A small document applies at once [lop_small_sync]

- **WHEN** a rotation runs on a 64 × 48 document
- **THEN** no progress window appears, the job delay seam is never reached, and the rotation has applied when the command returns

### Requirement: A job reports the progress its engine can count

A whole-document job SHALL report real progress when its engine can count its
work: resampled rows for Image Size, layers for Rotate and Flip, and composited
rows for Merge and Flatten. A job whose engine cannot weigh its passes (Canvas
Size and the mode, depth and HDR conversions) SHALL show a busy bar. A counted
engine SHALL check the cancel flag at each row, layer or band, and a cancelled
engine SHALL report that it was cancelled.

#### Scenario: Counted engines report every unit [lop_counted_progress]

- **WHEN** Image Size, Rotate, Flip, Merge Down and Flatten each run under a job token on a three-layer document
- **THEN** each reaches the last value before completion (999), so every declared unit was reported

#### Scenario: A job that cannot count shows a busy bar [lop_busy_bar]

- **WHEN** a cancellable job reports a negative progress value, as an 8-to-16-bit conversion on a large document does
- **THEN** the visible progress window shows a busy bar and never a percentage

#### Scenario: A cancelled engine stops early [lop_engine_stops]

- **WHEN** Image Size, Rotate and Flatten run under a job token whose cancel flag is already set
- **THEN** each reports that it was cancelled, without resampling a row, rotating a layer, or compositing a band
