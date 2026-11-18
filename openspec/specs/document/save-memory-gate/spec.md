# document/save-memory-gate Specification

## Purpose
Gate every save by what it holds at once, so a large tiled document saves under
a memory limit and a format that needs the whole image is refused cleanly
when it does not fit (#1320).

## Requirements

### Requirement: A native save reserves what it holds

Before it starts, a native PSD/PSB save SHALL reserve its working bytes with
the memory governor for the whole save: the tile tables of its copy of the
document, one compositor band, and the encoder's working rows. It SHALL admit
the merged image's tiles against the governor's room plus the scratch disks'
free space, without reserving them. The estimate SHALL NOT scale with the
document's own pixel bytes. A refusal of either SHALL be reported as a memory
refusal.

#### Scenario: A large tiled save is not estimated as two documents [smg_estimate]

- **WHEN** the save cost of a 17000² document with four dense layers and a Background is estimated
- **THEN** its working bytes are below 512 MiB, while the previous estimate was over 10 GiB

#### Scenario: A 2 GiB host admits the save [smg_admitted]

- **WHEN** the host is pinned to a 2 GiB cgroup with 1.5 GiB in use and 4 GiB of scratch free, and that document's save is gated
- **THEN** the save is admitted

#### Scenario: A host too small for the working bytes refuses it [smg_refused]

- **WHEN** the host is pinned to a cgroup whose room is below the save's working bytes
- **THEN** the save is refused as a memory refusal and nothing is written

### Requirement: Every raster save is gated by its format's cost

Every raster save path (the engine Save As formats, GIF, and the Qt JPEG and
WebP writers) SHALL reserve its format's cost before it builds a composite. A
streaming format SHALL reserve its band and encoder rows, and admit the tiled
composite as spillable. A whole-image format SHALL also reserve the flat image
buffers it builds and an upper bound of its encoded file. A refusal SHALL be
reported as a memory refusal.

#### Scenario: A whole-image format is refused when its buffers do not fit [smg_whole_refused]

- **WHEN** the host's room covers a streaming format's cost but not a GIF's whole-image buffers for the same document, and the document is saved as TIFF and as GIF
- **THEN** the TIFF save is admitted, and the GIF save is refused as a memory refusal with no file written

### Requirement: The estimates cover the measured peaks

Each save estimate SHALL cover the heap peak its save measures in the heap-peak
harness.

#### Scenario: Save estimates cover their peaks [smg_covers_peak]

- **WHEN** the harness measures the PSD, PNG, TIFF and GIF saves of its 4250² document
- **THEN** each heap peak is at most its gate's estimate

#### Scenario: M24 saves under a 2 GiB limit [smg_m24]

- **WHEN** M24 runs at 17000² under `MemoryMax=2G`: open the four-layer PSD, pan, fill one layer, save
- **THEN** every step succeeds, including the save, and the layer digests equal the unconstrained run
