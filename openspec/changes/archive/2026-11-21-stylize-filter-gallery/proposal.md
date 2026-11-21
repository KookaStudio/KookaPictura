# Stylize Filters in the Filter Gallery

## Why

`Filter ▸ Filter Gallery…` lists only Glowing Edges under Stylize. Diffuse,
Emboss, Extrude, Find Edges, Solarize, Tiles, Trace Contour, and Wind are absent
even though `docs/06-filters/stylize-filters.md` documents all nine as gallery
members and all nine have menu entries and engine kernels. The catalogue rows
for the eight missing filters leave `gallery` unset, and the dialog derives
gallery membership and order from that rank, so they never appear (#596,
parity register FILT-5).

## What Changes

- Give Diffuse, Emboss, Extrude, Find Edges, Solarize, Tiles, Trace Contour, and
  Wind catalogue gallery ranks so the Stylize category contains all nine
  filters.
- Order the Stylize category as CS6 does: Diffuse, Emboss, Extrude, Find Edges,
  Glowing Edges, Solarize, Tiles, Trace Contour, Wind. Renumber the Texture
  ranks that follow so the category order stays intact.
- Extend the Filter Gallery Qt tests to assert the Stylize membership and order
  and to render every Stylize thumbnail.
- Update `ROADMAP.md` and the parity-gap register.

Out of scope: gallery membership in other families, 8-bit-only gallery gating,
and Smart Filter grouping of gallery stacks.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `imaging/filter-gallery`: the Stylize category membership and count in the
  gallery dialog.

## Impact

- `crates/pictura-filters/src/catalogue/rows/stylize.rs`: gallery ranks for the
  eight missing Stylize rows and the shifted Texture rows.
- `crates/pictura-app/cpp/tests/tst_filter_gallery.cpp`: Stylize category and
  thumbnail coverage.
- `ROADMAP.md` and `docs/dev/doc-code-parity-gaps.md`: mark FILT-5 shipped.
- No dialog, FFI, or engine-kernel changes: the existing rank-driven gallery
  path picks the filters up.
