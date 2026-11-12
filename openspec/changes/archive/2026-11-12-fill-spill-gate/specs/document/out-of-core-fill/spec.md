## ADDED Requirements

### Requirement: A fill is gated on RAM plus scratch

A Gradient, Paint Bucket, Edit > Fill or Edit > Stroke of a pixel layer SHALL
be gated on two terms. Its working set SHALL fit the governor's room: the
flood masks of a Paint Bucket or Stroke, and a dense refresh composite of the
changed region where the refresh builds one. The tiles it forks from history
SHALL fit that room plus the free space on the usable scratch volumes. A
refresh SHALL be charged as a dense composite only for an authoritative
composite, a document of at most 16 Mpx, or a region of at most 4 Mpx. When
the forked tiles do not fit, the fill SHALL be refused with a scratch-full
refusal, the document and history SHALL be unchanged, and the fill refusal
message SHALL name the scratch disks.

#### Scenario: A large fill spills instead of refusing [ocf_gate_spills]

- **WHEN** the cost of a Gradient over a full-canvas layer of a 17000 × 17000 RGB/8 document is gated with the governor's room pinned at 1.75 GiB and 64 GiB of scratch available
- **THEN** the fill is admitted, although its forked tiles plus a 6-byte-a-pixel refresh exceed the room

#### Scenario: A small document's refresh stays charged to RAM [ocf_small_refresh]

- **WHEN** the cost of a Gradient over a full-canvas layer of a 2000 × 2000 RGB/8 document is computed
- **THEN** its working set includes the 6-byte-a-pixel refresh of the whole canvas

#### Scenario: Every fill commits with its tiles on scratch [ocf_fills_commit]

- **WHEN** a Gradient, a Paint Bucket, an Edit > Fill and an Edit > Stroke each run on a full-canvas layer of a 4100 × 4100 document, with room for their working set but not for their old RAM charge, and 4 GiB of scratch available
- **THEN** each adds one history state and no memory refusal is recorded

#### Scenario: Full scratch disks refuse a fill with their own message [ocf_scratch_full]

- **WHEN** a Gradient runs on the same document with room for its working set but not its forked tiles, and no scratch available
- **THEN** it is refused, the scratch-full refusal is recorded, history is unchanged, and the fill refusal message names the scratch disks

### Requirement: A large-document fill runs as a cancellable job

An interactive Gradient, Paint Bucket, Edit > Fill or Edit > Stroke of a pixel
layer on a document of 4 MP or more SHALL be gated on the GUI thread and then
run its engine on a worker as a cancellable job with a busy bar. The document
and history SHALL stay unchanged until the job lands. A completed job SHALL
record the same single history state as the synchronous fill. A cancelled job
SHALL leave the document pixel-identical and the history count and index
unchanged. A fill of a mask or Quick Mask, a fill on a smaller document, and a
fill not issued from a tool or menu SHALL apply at once.

#### Scenario: A large gradient runs as a job [ocf_fill_job]

- **WHEN** a Gradient is drawn with the Gradient tool on a 2400 × 1800 document whose job is held by the test seam
- **THEN** the progress window appears with a busy bar, and on landing one `"Gradient"` history state has been added and the layer's pixels changed

#### Scenario: A cancelled fill changes nothing [ocf_fill_cancel]

- **WHEN** Edit > Fill runs on a 2400 × 1800 document and Cancel is pressed while its job is held
- **THEN** the history count and index and the displayed pixels are unchanged

#### Scenario: A small document fills at once [ocf_fill_small_sync]

- **WHEN** a Gradient is drawn with the Gradient tool on a 64 × 48 document
- **THEN** no job is pending, no progress window appears, and the `"Gradient"` state has been recorded when the release returns
