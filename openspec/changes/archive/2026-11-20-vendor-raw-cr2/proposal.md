# Proposal: vendor raw containers, part 1 — the shared base and Canon CR2 (#399)

## Why

Raw open (#86) reads DNG only (#396). Most cameras write their own container,
so #399 adds in-house parsers for them, one container family per pull request
(CR2 → NEF → ARW → ORF/RW2 → RAF with an X-Trans demosaic → CR3). This first
part lays the shared base every later container uses and adds Canon CR2, a
TIFF container whose sensor plane is lossless JPEG, the codec #396 already
decodes.

## What changes

- `pictura_codec::sniff_raw(bytes)` names a raw container by its signature
  (`RawFormat::Dng`, `RawFormat::Cr2`), and `pictura_codec::read_raw(bytes,
  budget)` reads any supported one into the same `DngImage` that `read_dng`
  returns, so the demosaic (#397) and the File ▸ Open route (#400) take every
  container alike. Anything else is refused with `PsdError::Unsupported`.
- CR2: the raw IFD named by the header, compression 6 lossless JPEG with one
  to four components, the vertical slices of tag `0xC640` put back into
  columns, the active area from the Canon maker note's sensor information,
  black from the masked border, white from the frame precision. Make, model
  and orientation are reported; small/medium raw (subsampled) is refused.
- Decode only: CR2 files carry no colour matrix, so `DngImage::color` stays
  empty. Where per-camera matrices come from is a follow-up issue; until then
  `as_shot_transform` refuses such an image as it refuses a DNG without
  `ColorMatrix`.

## Capabilities

- New: `codec/vendor-raw-containers`.

## Provenance

Built from public descriptions of the CR2 layout (the TIFF 6.0 and Exif
specifications, ITU T.81, and public write-ups of the CR2 header, slices and
Canon sensor-information tag) and observation; no code or tables from dcraw,
LibRaw or rawspeed. Fixtures are synthetic, written by the tests; real camera
files stay outside the repository and are checked opt-in.
