# document/in-place-history Specification

## Purpose
A private current-state history model for in-place edits that avoids taking
full document snapshots.

## Requirements

### Requirement: Band-shared current state

The history SHALL hold the state at its cursor in band-granular planes that
share every unchanged band with the live document. Opening a document SHALL NOT
copy tracked-plane pixels for the history and SHALL assign fresh stamps to both
the live document and the history cursor state. Capturing a state SHALL compare the
live document with the current state band by band, SHALL skip bands whose
allocations are shared, and SHALL scan the 64×64 tiles of the remaining bands
across all cores, copying only the changed tiles into the current state. A
capture SHALL skip only bands that are still shared: a contiguous read of a
plane followed by another write adopts the materialized view as the plane's base,
so its bands no longer match the current state's and the capture scans that plane
fully; untouched planes still skip every band. A stroke SHALL write through the band
writer, so a capture SHALL scan only the bands the stroke touched. A write to the
live document after a capture SHALL fork at most the bands it touches, so the
current state keeps its copy of those bands and every other band stays shared.

#### Scenario: Open shares bands with the live document [iph_shared_bands]

- **WHEN** a document opens and a stroke is committed and undone
- **THEN** every restored state is byte-identical to its capture and the history
  shares every untouched band with the live document

#### Scenario: Capture scans only changed bands [iph_capture_skips_shared_bands]

- **WHEN** a dab touches one band and the state is captured with no contiguous
  read of that plane in between
- **THEN** only that band's 64×64 tiles are compared and every other band remains
  one allocation shared by the live document and the current state; a contiguous
  read of the plane before the capture materializes it, so its bands no longer
  match and the capture scans that plane fully, while untouched planes still
  skip their bands

#### Scenario: A live write forks only touched bands [iph_fork_on_write]

- **WHEN** the live document is written after a capture
- **THEN** only the written bands are copied, and the current state's bytes for
  those bands are unchanged

### Requirement: In-place undo with restored damage

Undo, redo and jump SHALL bring the live document to the target state by
applying the stored tiles to it in place when the target is in the same
geometry segment, and SHALL report the document rectangle whose composite
changed, or the whole document when the change cannot be bounded (a geometry or
document-level change). The canvas SHALL refresh only the reported rectangle.

#### Scenario: Undoing a stroke reports its tiles [iph_undo_damage]

- **WHEN** a brush stroke is committed and undone
- **THEN** the live document equals the pre-stroke state byte for byte and the
  reported rectangle covers the stroke's composite tiles, not the document

#### Scenario: Crossing a geometry change reports the whole document [iph_geometry_damage]

- **WHEN** an undo crosses a resize, crop, or layer-structure change
- **THEN** the live document equals the target state exactly and the reported
  rectangle is the whole document

### Requirement: One materialized copy per history

The anchor of the geometry segment that holds the cursor SHALL keep only
metadata, its pixels being the band-shared current state with the segment's
deltas reverted, so a document's pixels are retained by the history once rather
than twice. Every other anchor SHALL keep its full state. The current state
SHALL share every unchanged band with the live document and SHALL copy only the
bands a capture or restore changed. A stroke SHALL write through the band
writer, so a capture SHALL scan only the bands the stroke touched.

#### Scenario: Reaching the oldest state from the private copy [iph_hollow_anchor]

- **WHEN** several strokes are captured after an open and undone back to the
  oldest state, then redone
- **THEN** every state is byte-identical to its capture and the oldest state was
  rebuilt from the band-shared current state, not from a second retained copy
