# Design: port-photorust-displace

## Model

Ported from perfecto25/photorust `core/src/filters/distort.rs::displace`.

- **Geometry.** Destination pixel centre `(x, y)` samples the source at
  `(x + (h − 128) × H/100, y + (v − 128) × V/100)`, where `h` and `v` are the
  map's horizontal and vertical samples and `H`, `V` the scales in percent.
  That is the contract's `(v − 128)/128 × scale` with 100% reaching 128 px
  (127 at white, 128 at black). Photorust scaled 100% to half the image; the
  contract wins. The source is read bilinearly through the shared `remap`.
- **Sign.** A light sample reads the source to the right and below, so the
  picture moves up and to the left. ImageMagick's `-compose displace` uses the
  same direction, and so did photorust. *(inferred for CS6)*
- **Fit.** `StretchToFit` reads the map at `(x / w × mw − 0.5, y / h × mh − 0.5)`,
  bilinear and clamped, so a small map stretched over a large layer bends
  smoothly; at equal sizes this is the map's own pixel. `Tile` reads the map
  pixel `(⌊x⌋ mod mw, ⌊y⌋ mod mh)`.
- **Edges.** `wrap` maps to the shared `EdgeMode` (`Wrap` / `Clamp`).
- **Ceiling.** The map is fitted to the layer buffer; CS6 fits it to the
  selection's bounds (`ponytail:` in `displace.rs`).
- **No-op.** A map that is all 128, or both scales 0, returns before touching
  the buffer, so a neutral run is bit-exact on translucent pixels.
- **Alpha.** The colour pass never writes alpha; Displace is in
  `filter_preserves_opacity`'s moving list, so the grey alpha pass displaces
  the layer's alpha the same way.

## Map file

- **Reading.** `read_psd_with(.., Policy::Off)`: the samples are distances, so
  an embedded profile is not applied. The merged composite is used when the
  file has one, else the layers are composited on the CPU.
- **Channels.** Plane 0 drives x and plane 1 drives y; a one-channel file
  (Grayscale) gives its plane to both, which shifts along the diagonal the two
  scales define. Bitmap mode is refused.
- **Ceiling.** CMYK, Lab, and Indexed files are normalized to RGB on read, so
  their "first two channels" are red and green rather than the file's own
  (`ponytail:` in `displace_maps.rs`).

## Carrying the map

A filter is a kind plus number slots, and the map is an image. CS6 asks for the
map after the dialog's OK, so:

1. The dialog edits four visible slots plus a `Hidden` fifth (the map id,
   default 0).
2. After OK, `runFilterDialog` opens a file dialog, calls
   `filter_displace_map_load(path)`, and writes the id into the last slot.
   Cancel or an unusable file abandons the filter without a history state.
3. `filter_from_kind_params("displace", ..)` resolves the id through the store.
   Id 0 is `DisplaceMap::neutral()`, so the "every kind defaults from an empty
   slot list" invariant holds and a dialog's defaults preview nothing. An id
   the store no longer holds resolves to `None`, so the commit is refused.
4. Last Filter reuses the stored slots, so it reuses the map, as CS6 does.
   Last Filter Settings reopens the dialog and asks for a map again.

The store keeps the eight most recent maps for the process (`ponytail:`); none
survive a restart.

## Oracle

ImageMagick's `MAP -compose displace -define compose:args=XxY -composite`
reaches `X` px at full white and centres on 127.5. With `X = 1.275 × scale`,
Repeat Edge Pixels matches within max 5 / mean 0.5 at 4×4 and 6×−5 on a
ramp map (9 at 8×−6, from the half-level centre offset across the test
image's 170-level checker edges). Wrap Around is not diffed: Pictura's
`remap` wraps the sample position but blends the edge pixel with itself,
where ImageMagick's tiled virtual pixels blend across the seam (57 of 768
samples over 9, max 125).
