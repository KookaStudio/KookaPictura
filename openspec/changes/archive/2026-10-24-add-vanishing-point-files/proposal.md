# Proposal: add-vanishing-point-files

## Why

Issue #918, the follow-up to #349. The Vanishing Point dialog keeps its
planes on the open document in memory only. CS6 keeps them in saved PSD,
TIFF and JPEG files (`docs/06-filters/vanishing-point.md`, Data-model
impact), so reopening a file goes back to the same planes.

## What Changes

- **A plane encoding.** `pictura-filters` `VpPlanes` gains `to_bytes` and
  `from_bytes`: a versioned, big-endian record of what the user drew (root
  corners, anchor, unit, grid size; related planes' parent, edge, angle,
  side, span and last outline). Decoding re-solves the planes.
- **A project-owned image resource.** `pictura-codec` gains the planes
  resource: id 4900 in Photoshop's plug-in range, marked by the Pascal name
  `Kooka Vanishing Point`. A block with that id and another name is not
  ours and is left alone. Every other resource stays byte-for-byte. CS6's
  own block is undocumented, so Photoshop does not read these planes.
- **JPEG and TIFF carry the Photoshop resource block both ways.**
  - Saving writes IPTC-IIM and the planes into one block: APP13 in a JPEG
    (split across segments past 64 KB) and tag 34377 in a TIFF.
  - Opening a JPEG or TIFF now reads that block into the document's image
    resources, less any ICC profile block (the opened pixels are not
    converted from it). Before, a raster open read no metadata at all.
- **The app saves and restores the planes.**
  - PSD, PSB, TIFF and JPEG saves and the crash-recovery snapshot write the
    session's planes. Installing a document reads them back when they fit
    its canvas.
  - OK that changes only the planes marks the document modified, with no
    history state.

## Capabilities

### New Capabilities

- `codec/vanishing-point-resource`: the plane encoding, the resource, and
  the JPEG and TIFF Photoshop block.

### Modified Capabilities

- `imaging/vanishing-point`: a new requirement covers saving and reopening
  the planes and the modified mark.

## Impact

- `pictura-filters`: `distort/vanishing_point/persist.rs`.
- `pictura-codec`: `image_resources.rs` and `raster_metadata/*`.
- `pictura-app`:
  - `filter_tools/vanishing_point*.rs`, `document_session/session.rs`
  - the save in `impl_core.rs`, `impl_core/recovery.rs`, the raster open in
    `open_jobs.rs`
- Tests: Rust unit tests in all three crates and the new
  `tst_vanishing_point_files`.
