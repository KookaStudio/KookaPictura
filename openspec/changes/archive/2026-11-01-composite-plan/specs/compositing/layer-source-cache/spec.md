# Spec Delta

## MODIFIED Requirements

### Requirement: Cache lookups on the worker pool never wait on an in-flight build

A layer-source cache lookup that runs on a rayon worker SHALL NOT block on a
build of the same key that another lookup has started, on any thread, because a
build may run parallel work on the same pool. Such a lookup SHALL build the
artifact itself; when several lookups of one key build concurrently, the first
published artifact SHALL be the one every caller receives and the one the cache
retains. A lookup outside the pool SHALL wait for an in-flight build of its key
instead of building a duplicate, and a build that panics SHALL release its
claim so a later lookup of the key builds it. A banded composite SHALL still
build each layer's whole-layer effect passes exactly once before the bands
execute.

#### Scenario: A same-key lookup run mid-build completes [lsc_steal]

- **WHEN** a pool worker building a cached artifact runs a queued job that looks
  up the same key before the build finishes
- **THEN** both lookups complete, and both return the same published artifact

#### Scenario: A same-key lookup in a stolen half of the build completes [lsc_steal_cross]

- **WHEN** a build joins on parallel work, another pool worker runs the joined
  half, and that half looks up the key being built while the build waits for it
- **THEN** both lookups complete, and both return the same published artifact

#### Scenario: Concurrent lookups outside the pool build once [lsc_dedupe]

- **WHEN** eight threads outside the worker pool look up one missing key at
  the same time
- **THEN** the artifact is built exactly once

#### Scenario: A panicked build does not wedge its key [lsc_panic]

- **WHEN** a lookup's build panics
- **THEN** the panic reaches that caller, and the next lookup of the key builds
  and returns the artifact

#### Scenario: A crafted huge glow composites in bands without hanging [lsc_huge_glow]

- **WHEN** a 512×512 document with an inner glow of size and choke `1e30` is
  composited in bands on the shared pool
- **THEN** the composite finishes within 15 seconds and leaves every pixel
  outside the layer equal to the plain composite

#### Scenario: A cold banded composite builds one whole-layer pass [lsc_band_once]

- **WHEN** a tall sub-region of a styled document is composited in bands with a
  purged cache
- **THEN** the layer's whole-layer pass is built exactly once and the output
  equals the whole composite
