## MODIFIED Requirements

### Requirement: Whole-document commands run as cancellable jobs

An interactive Image Size, Canvas Size, Rotate (the quarter turns and
Arbitrary), Flip, mode conversion, bit-depth or HDR conversion, Merge Down,
Merge Layers, Merge Visible, Merge Clipping Mask, or Flatten Image on a
document of 4 MP or more SHALL run its engine on a worker as a cancellable job,
while the document and history stay unchanged until the job lands. A completed
job SHALL record the same single history state as the synchronous command. A
cancelled job SHALL leave the document pixel-identical and the history count
and index unchanged. A command on a smaller document, or one not issued from
the menus or panels, SHALL apply at once.

#### Scenario: A large rotation runs as a job [lop_whole_image_job]

- **WHEN** Image > Image Rotation > 90° CW runs on a 2400 × 1800 document whose job is held by the test seam
- **THEN** the progress window appears, and on landing the document is 1800 × 2400 with one added `"Rotate"` history state

#### Scenario: A large arbitrary rotation runs as a job [lop_arbitrary_rotate_job]

- **WHEN** Image > Image Rotation > Arbitrary runs at 15° on a 2400 × 1800 document whose job is held by the test seam
- **THEN** the progress window appears, the document is unchanged while the job is held, and on landing the canvas has grown to the rotated bounds with one added `"Rotate"` history state

#### Scenario: A cancelled whole-document command changes nothing [lop_whole_image_cancel]

- **WHEN** Flatten Image runs on a large two-layer document and Cancel is pressed while its job is held
- **THEN** the layer count, the history count and index, and the displayed pixels are unchanged, and the command applied afterwards adds one state

#### Scenario: A merge job reports the merged layer [lop_merge_job]

- **WHEN** a Merge Down job lands
- **THEN** the merged layer is the active target and the reported outcome names its path and the two layers it replaced

#### Scenario: A small document applies at once [lop_small_sync]

- **WHEN** a rotation runs on a 64 × 48 document
- **THEN** no progress window appears, the job delay seam is never reached, and the rotation has applied when the command returns
