## ADDED Requirements

### Requirement: Rows beyond the path answer without an edge scan

A vector-mask sample in a row above the path's first crossing row or at or
below its last SHALL report outside at once, without walking the path's edges,
whatever clip built the row table. Only a row inside the path's row span but
outside the cached table SHALL fall back to scanning the edges.

#### Scenario: Rows past the path are outside [vms_beyond]

- **WHEN** a rectangle mask whose table was built for a two-row clip is sampled
  in rows above, below, and inside the path, under even-odd and non-zero
- **THEN** rows inside the path (cached or not) are inside and every row beyond
  it is outside
