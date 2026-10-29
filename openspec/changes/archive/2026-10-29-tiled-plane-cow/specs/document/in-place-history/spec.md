# Spec Delta

## REMOVED Requirements

### Requirement: Private current state

**Reason**: The private copy existed because a plane could not be shared at a
granularity finer than the whole plane. Band-granular planes make the copy
unnecessary and the requirement's central invariant (no live plane is shared
with history) false: untouched bands are now shared by design.

**Migration**: None for users. The public `History` API, state order, labels,
depth, and byte-exact restore are unchanged; only the storage relationship
between the live document and the current state changes.

## ADDED Requirements

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

## MODIFIED Requirements

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
