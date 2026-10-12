## ADDED Requirements

### Requirement: Liquify workspace

`Filter ▸ Liquify…` (Shift+Ctrl+X) SHALL be enabled whenever filters are and SHALL open the Liquify workspace over the active layer, titled `Liquify (<document> @ <zoom>%)`. Basic mode SHALL show Forward Warp, Reconstruct, Pucker, Bloat, Push Left, Hand and Zoom with Size (default 100, up to 15,000) and Pressure (default 100); Advanced Mode SHALL add Twirl Clockwise, Mirror, Turbulence, Freeze Mask and Thaw Mask, Density (50), Rate (80) and Turbulent Jitter (50), Reconstruct…, the Mask Options and Show Mask. Brush strokes SHALL preview in the workspace without changing the document. Cancel SHALL leave the document and history untouched; OK with no distortion SHALL record nothing; OK with a distortion SHALL commit one filter history state. The `liquify` kind SHALL take one slot naming the kept mesh; an empty list SHALL be an identity mesh and an id no longer kept SHALL be refused. Load Last Mesh SHALL reload the mesh of the last OK, and Last Filter Settings SHALL reopen the workspace with it.

#### Scenario: Basic mode shows CS6's short column

- **WHEN** the workspace opens
- **THEN** only Forward Warp, Reconstruct, Pucker, Bloat, Push Left, Hand and Zoom are shown, Size and Pressure read 100, and Advanced Mode reveals the remaining tools and options

#### Scenario: A drag previews, Cancel discards, OK commits

- **WHEN** a Forward Warp drag is made and Cancel is pressed
- **THEN** the workspace preview changed but the document and history did not; repeating the drag and pressing OK adds one history state with the `liquify` kind and a nonzero mesh id

#### Scenario: Load Last Mesh reapplies the committed mesh

- **WHEN** the workspace opens after a committed Liquify and Load Last Mesh is pressed
- **THEN** the preview shows that distortion again
