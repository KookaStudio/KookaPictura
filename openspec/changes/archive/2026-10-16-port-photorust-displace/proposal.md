# Proposal: port-photorust-displace

## Why

Issue #335 (part of the #167 bundle, under the #183 port umbrella). CS6's
`Filter ▸ Distort ▸ Displace` moves each pixel by an amount read from a second
image, the displacement map. Photorust has the kernel
(`core/src/filters/distort.rs::displace`), but Kooka has no `Filter::Displace`,
no `displace` kind, and the menu row is a disabled stub. The docs contract
(`docs/06-filters/distort-filters.md`) fixes the model: channels 1 and 2 of a
flattened map PSD drive the x and y shifts, `(v − 128) / 128 × scale`, up to
128 px at 100%; the map is stretched to fit or tiled; undefined areas wrap
around or repeat edge pixels.

## What Changes

- `pictura-filters` gains `DisplaceMap` (two 8-bit planes, `neutral()`),
  `DisplaceFit { StretchToFit, Tile }`, and
  `Filter::Displace { map, horizontal_scale, vertical_scale, fit, wrap }`.
  The kernel is ported to `photorust/displace.rs` on the shared `remap`, with
  the contract's reach (128 px at 100%) rather than photorust's half image,
  and a bilinear map lookup when stretching. Displace joins the
  alpha-moving allowlist.
- `pictura-app` keeps loaded maps in a small store
  (`cxxqt_object/filter_tools/displace_maps.rs`): `filter_displace_map_load(path)` reads a
  flattened PSD (embedded profile ignored, Bitmap mode refused) and returns an
  id. The `displace` kind has 5 slots: Horizontal Scale, Vertical Scale,
  Displacement Map, Undefined Areas, and the map id. Id 0 is the neutral map.
- `Filter ▸ Distort ▸ Displace…` opens CS6's dialog (two scales, two radio
  groups, no preview). After OK a file dialog asks for the map; its id fills the
  hidden last slot. Last Filter replays the same map; Last Filter Settings asks
  again.
- `FilterControl::Hidden` carries one slot through the dialog without a widget,
  and `FilterCommandSpec::livePreview = false` turns a dialog's preview off.

## Capabilities

### New Capabilities

- `imaging/distort-filters` gains *Displace*: the map lookup, the reach, the
  fit and edge modes, and the ImageMagick oracle.
- `imaging/filter-app-ui` gains *Displace dialog and map file*: the dialog, the
  post-OK map chooser, the slot order, and the map store.

## Impact

- `pictura-filters` (`lib`, `filter/types`, `photorust/{displace,dispatch,distort,mod}`),
  `pictura-render` (`filter_preserves_opacity`), `pictura-app` (`filter_map.rs`,
  `filter_tools.rs`, `displace_maps.rs`, `filter_commands.{h,cpp}`,
  `filter_param_controls.{h,cpp}`, `filter_preview_dialog.cpp`,
  `frame_menus_filter.cpp`), plus `tst_displace` and `tst_filter_menu`.
- **Oracle:** Displace is the first Distort filter with a faithful
  ImageMagick operator (`-compose displace`); Repeat Edge Pixels is diffed at
  tolerance 8. `scripts/filter_oracle.py`, `mapping.rs`, and `tests/README.md`
  are updated.
- **Docs:** `docs/06-filters/distort-filters.md` records the defaults, the
  sign, the map handling, and the oracle, in a separate `TASK-ALLOWS-DOCS`
  commit.
