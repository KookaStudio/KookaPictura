# Design: lens-correction-smart-filter

Mirrors the Camera Raw smart filter (`pictura_raw`), which is the one
reference-grounded smart filter. Lens Correction's own filter id and key map
are not available, so this descriptor is **Pictura-only** (per the #934
decision): a document authored here re-opens here, and no cross-app
round-trip is claimed. The generic smart-filter machinery still gives
byte-for-byte preservation of keys we do not model.

## Descriptor

The smart filter carries the dialog's 16 slots as one descriptor under `Fltr`,
the same slot order the `lens-correction` kind and `LensCorrectionParams`
already use (Auto-tab checkboxes, Auto Scale, Edge, profile id, then the
Custom sliders). The class id and `Nm  ` are our own; `filterID` is a
Pictura-only constant that does not collide with the reference camera-raw id
2683.

| Slot | Meaning | Key | Type |
|---|---|---|---|
| 0 | Geometric Distortion | `Dstn` | Bool |
| 1 | Chromatic Aberration | `CAOn` | Bool |
| 2 | Vignetting | `VgnO` | Bool |
| 3 | Auto Scale Image | `AScl` | Bool |
| 4 | Edge (0 Extension, 1 Transparency, 2 Black, 3 White) | `Edge` | Long |
| 5 | profile id (0 none) | `Prof` | Long |
| 6 | Remove Distortion | `RDis` | Double |
| 7 | Fix Red/Cyan | `FRCY` | Double |
| 8 | Fix Green/Magenta | `FGMG` | Double |
| 9 | Fix Blue/Yellow | `FBYL` | Double |
| 10 | Vignette Amount | `VigA` | Double |
| 11 | Vignette Midpoint | `VigM` | Double |
| 12 | Vertical Perspective | `PerV` | Double |
| 13 | Horizontal Perspective | `PerH` | Double |
| 14 | Angle | `Angl` | Double |
| 15 | Scale | `Scal` | Double |

Decode returns the 16 slots, each defaulting to the descriptor's opening value
when its key is missing or the wrong type. `attach_lens_correction_filter`
builds the `SmartFilter` and calls the existing generic `attach_smart_filter`,
so a preserved `SoLd`'s other keys and any unmodeled `Fltr` keys survive a
re-edit (the entry's modelled keys are overlaid, new values winning).

## Render

`decode_smart_filter` gains `SmartFilterOp::LensCorrection(LensCorrectionParams)`:
decode the slots, then `LensCorrectionParams::from_slots` resolves the profile
id against the bundled profiles and gates its parts by the checkboxes. A slot
list that does not map (unknown profile id, wrong count, non-finite value)
returns `None`, so the layer stays on its proxy path exactly as an unknown
filter id does. `apply_op` clones the buffer and runs
`pictura_filters::lens_correction`, which preserves alpha (the chain buffer is
4-channel RGBA).

The compositor already renders the smart source through the chain for a
`SmartFiltered` route; the Lens Correction op slots in with no routing change.

## Commit and re-edit

A `lens-correction` commit whose active layer is an embedded Smart Object takes
the smart path (both the dialog commit and the executor/Last Filter paths):

1. Attach (or replace) the Lens Correction smart filter on the layer.
2. Leave the layer's proxy as the unfiltered source; the compositor renders the
   chain live, so disabling, clearing, or deleting the filter falls back to the
   source rather than to a baked correction.

One history state is recorded; the kind and its 16 slots are stored for Last
Filter and Last Filter Settings. A plain raster layer keeps the ordinary
destructive commit unchanged.

The dialog session reads its source from the Smart Object's embedded source
(not the baked proxy), so a re-edit previews the source plus the current
settings and never re-applies the baked correction. `lc_begin` also captures
the layer's stored slots; `lc_stored` hands them to the dialog so an empty
open prefills from the smart filter, as Camera Raw's dialog does.

ponytail: like the Camera Raw smart filter, the proxy is baked from the source
alone, so with several smart filters on one layer the proxy reflects only the
one just applied; the compositor still renders the whole chain, and the
dialog previews from the source.
