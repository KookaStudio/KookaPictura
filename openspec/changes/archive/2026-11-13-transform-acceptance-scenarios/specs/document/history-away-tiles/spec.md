## ADDED Requirements

### Requirement: A history capture does not read back evicted tiles

When a capture compares a changed plane tile by tile, a tile that is packed or
spilled on either side SHALL be recorded as changed over its whole tile rect
without restoring its samples; only tiles resident on both sides SHALL be read
to narrow the changed box or find identical samples. The delta SHALL keep the
old tile by reference, so undo restores the same samples either way.

#### Scenario: An evicted tile is wholly changed and stays evicted [hist_away_tile]

- **WHEN** one sample of a dense tile changes, both tiles are then evicted, and
  the capture's comparison runs
- **THEN** while resident the change box is that one sample; once evicted it is
  the whole tile rect, and neither tile has been read back
