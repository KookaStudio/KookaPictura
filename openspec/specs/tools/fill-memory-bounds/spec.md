# tools/fill-memory-bounds Specification

## Purpose
Keeps the fill family (Paint Bucket, Fill, Stroke, Gradient) within the
selection's box on large documents, makes the selection a wall for the
bucket's flood, and refuses a fill whose peak memory the host cannot hold.

## Requirements

### Requirement: The Paint Bucket floods inside the selection's box

With a selection, the Paint Bucket SHALL test, flood and fill only the pixels
of the selection's bounding box, reading the layer's or composite's planes
directly, without a document-sized copy of the source or a seed plane. The
match of each pixel MUST equal the per-channel inclusive tolerance test on the
document-space surface. Without a selection the box SHALL be the canvas.

#### Scenario: A boxed flood reads the planes as the surface does [fmb_flood_box]

- **WHEN** layers inside the canvas, offset past its edge, and without a
  transparency plane, and a planar composite, are flooded over the canvas and
  over an inner box, contiguous and not
- **THEN** every match equals the interleaved surface's test, the box flood
  equals the canvas flood cropped to the box, and the single-seed flood equals
  the seed-plane flood

### Requirement: The selection is a wall for the contiguous flood

A pixel outside the selection SHALL NOT match, so the contiguous flood MUST NOT
reach a selected region through unselected pixels. A seed outside the selection
SHALL flood nothing. An unselected pixel SHALL NOT soften the anti-aliased edge
of the pixels next to it. CS6's behaviour is not documented; this rule is an
owner decision.

#### Scenario: Two selected rooms fill one at a time [fmb_wall]

- **WHEN** two rectangles of a uniform layer are selected, joined by
  unselected pixels of the same colour, and the bucket clicks one of them
- **THEN** only the clicked rectangle fills when contiguous, both fill when not,
  and the gap between them is untouched

#### Scenario: The selection edge is not softened [fmb_antialias_wall]

- **WHEN** a uniform layer is flooded with anti-aliasing inside a diagonal
  selection
- **THEN** every selected pixel of the mask is fully covered

### Requirement: The fills write only the bands their box crosses

The Fill, the Gradient and the Stroke SHALL confine their visit and their
writes to the selection's box (the Stroke's grown by its reach and one pixel),
forking only the plane bands that box crosses. Their pixels and changed
rectangle MUST equal the whole-document computation's.

#### Scenario: A fill without a mask covers its box [fmb_fill_box]

- **WHEN** a box is filled once with no mask and once with a full mask
- **THEN** the planes and changed rectangles are identical

#### Scenario: A windowed stroke equals the whole-document stroke [fmb_stroke_window]

- **WHEN** rectangles inside the canvas and against its edges and a soft
  ellipse are stroked Inside, Center and Outside at 1, 3 and 6 px
- **THEN** each stroke's planes and changed rectangle equal the band built
  over the whole document

#### Scenario: A boxed gradient forks only its bands [fmb_gradient_box]

- **WHEN** a gradient is drawn through a selection with and without its box on
  a layer shared with history
- **THEN** the pixels and changed rectangle are identical and a band outside
  the box stays shared

### Requirement: A fill that cannot fit in memory is refused

Before it allocates, the Gradient, the Paint Bucket, the Fill and the Stroke
SHALL estimate their peak bytes (their box-sized planes and the bands they
fork) and SHALL refuse the edit, recording no history state, when the estimate
exceeds three quarters of the memory available now. The refusal SHALL be
reported as "not enough memory" on the status bar, and the next fill that fits
SHALL run. When the host's memory cannot be read, the gate SHALL NOT refuse.

#### Scenario: Each fill refuses what memory cannot hold [fmb_gate]

- **WHEN** the available memory is made smaller than any fill's estimate and a
  bucket click, a gradient, a Fill and a Stroke are tried, then the memory is
  restored and the Fill tried again
- **THEN** each is refused with the memory reason, the pixels and history are
  unchanged, and the restored Fill commits one state and clears the reason

#### Scenario: A bucket in a marquee on a 17000×17000 document [fmb_large_manual]

- **WHEN** a 17000×17000 document is opened on a host with about 7 GB, a
  rectangular marquee is drawn and the Paint Bucket clicks inside it, then a
  fill is tried with too little memory free
- **THEN** the fill completes without the process being killed, and a fill that
  cannot fit is refused with the memory message instead of crashing
