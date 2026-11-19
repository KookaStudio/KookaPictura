# codec/merged-preview-band-buffers Specification

## Purpose
A lazily opened PSD's merged preview decodes a band's colour planes side by side on worker threads while the reading thread owns every band-sized buffer, so freed decode buffers are not kept in the workers' allocator arenas and a large open's memory peak stays at its live data.

## Requirements

### Requirement: A merged preview's band buffers are allocated by the reading thread

When a lazily opened document's merged preview decodes bands of its colour
planes side by side on worker threads, every band-sized buffer the decode
needs (the decoded rows and the packed bytes read from the file) SHALL be
allocated on the thread that reads the preview; the workers SHALL only fill
them. The preview's pixels MUST stay the merged image's.

#### Scenario: A whole-image read allocates no band-sized block on a worker [mpb_caller]

- **WHEN** a merged preview of a raw or RLE layered PSD reads every band of the image for the first time
- **THEN** no block of 64 KiB or more is allocated on a worker thread during the read

#### Scenario: The preview still matches the merged image [mpb_pixels]

- **WHEN** a region of a lazily opened layered PSD is read from its merged preview
- **THEN** its colour planes equal the eagerly read merged image's, and its alpha is opaque inside the image

#### Scenario: The fit-to-screen open of a large PSD does not keep freed band buffers [mpb_m12]

- **WHEN** the M12 profile (17000² layered PSD, lazy open, fit-to-screen first paint) runs on this build and on master
- **THEN** its peak memory is within master's before #1321 (about 647–665 MiB), and its first paint is no slower than about 2.4 s
