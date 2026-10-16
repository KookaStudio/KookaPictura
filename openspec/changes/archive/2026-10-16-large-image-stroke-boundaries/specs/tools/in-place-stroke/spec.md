## ADDED Requirements

### Requirement: Strokes paint the live document in place

A raster stroke SHALL write its dabs straight into the document it paints,
saving the original pixels of each 64×64 tile of the target layer before the
tile is first written, and SHALL blend every dab against those saved original
pixels so the committed pixels equal a stroke that read a full pre-stroke copy.
Cancelling a stroke SHALL restore the saved tiles. The cost of starting a stroke
and of its first dab SHALL NOT grow with the document size.

#### Scenario: In-place pixels equal the copying stroke [ips_exact_pixels]

- **WHEN** the same samples are painted in place and through a full pre-stroke
  copy, at several opacities, flows and modes, including overlapping dabs
- **THEN** the two documents are byte-identical

#### Scenario: Cancel restores the touched tiles [ips_cancel_restores]

- **WHEN** a stroke paints and is cancelled
- **THEN** the document is byte-identical to its state before the stroke began

#### Scenario: The first dab on a large document is dab-sized [ips_first_dab]

- **WHEN** the `large_document_stroke_profile` paints on a 16507×16196 document
- **THEN** the first dab copies no layer plane: it costs milliseconds, against
  the hundreds of milliseconds a 267 MB plane copy costs
