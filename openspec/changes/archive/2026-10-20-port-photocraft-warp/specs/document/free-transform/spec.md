## REMOVED Requirements

### Requirement: Mesh warp op

**Reason**: Replaced by photocraft's warp resampler (`document/transform-warp-engine`):
an adaptive forward-mapped triangle mesh driven by a `pictura_core::warp::Warp`
instead of a fixed 16×16 cell net with row/column distortion.
**Migration**: Call `transform_layer_warp(doc, path, &Warp, interpolation)`.

### Requirement: Preset warp style mesh generation

**Reason**: The Patchy-derived preset nets are replaced by photocraft's styles,
fitted to Photoshop renders (`pictura_core::warp::WarpStyle`, `StyleWarp`).
**Migration**: Use `Warp::preset(style, bend, bounds)`; `WarpStyle::id` and
`WarpStyle::parse` replace `id` / `from_id`.

### Requirement: Warp preset command and dialog

**Reason**: Warp is now a Free Transform mode with an on-canvas mesh and
options-bar fields (`document/transform-session-warp`), as in CS6.
**Migration**: `Edit > Transform > Warp` opens a session in Warp mode; the style
and values are set on the options bar.
