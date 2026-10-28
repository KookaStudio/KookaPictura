# Proposal: smart-filter-apply

## Why

Issue #984. A Filter-menu filter applied to a Smart Object runs the destructive
path today: it overwrites the proxy pixels, fails outright on an object with no
proxy, and has no visible effect once a smart-filter chain exists, because the
compositor renders the chain instead of the proxy. CS6 instead appends a
non-destructive Smart Filter (`docs/05-layers/smart-filters.md`). The model,
renderer, panel operations, and PSD round-trip already exist; only Camera Raw
and Lens Correction can be decoded, and only those two are attached by the app.

## What Changes

- A generic Pictura-only smart-filter record (`filterID` "PTFL") carries one
  filter, or a Filter Gallery stack as one "Filter Gallery" entry, as catalogue
  kinds and their full-resolution slots.
- `pictura-filters` names which kinds may become a Smart Filter: every kind but
  Liquify, Vanishing Point, and the resource-bound Lens Blur, Displace, and
  Adaptive Wide Angle.
- `pictura-render` decodes and renders the record through the destructive
  path's alpha handling, and gains an apply op that appends the record,
  turning an active selection into the filter mask when the object has none.
- `pictura-codec` gains an append variant of the generic attach (a same-id
  stack), a filter-mask enable setter, and authors the `FEid`/`FMsk` filter
  mask on save.
- `pictura-app` routes the Filter menu, the Filter Gallery, Last Filter, and the
  control server to the smart apply on an embedded Smart Object; the dialog
  previews non-destructively; excluded filters and Image > Adjustments are
  greyed out, and every destructive path refuses a Smart Object.

**Out of scope** (follow-up issues): Shadow/Highlight and Variations as Smart
Filters, double-click re-edit of a generic entry, per-filter blend mode and
opacity.

## Capabilities

### New Capabilities

- `imaging/smart-filter-apply`: applying a Filter-menu filter or a Filter
  Gallery stack to a Smart Object as a Smart Filter.

### Modified Capabilities

- `codec/smart-filter-masks`: the filter mask is authored on save.
- `codec/psd-smart-filters`: the generic attachment gains an append variant.

## Impact

- `pictura-codec` (`pictura_filter/`, `pictura_raw/mod.rs`, `smart_filter.rs`,
  `filter_effects.rs`, `write.rs`), `pictura-filters` (`catalogue`),
  `pictura-render` (`smart_filter.rs`, `document_ops/smart_filter_apply.rs`),
  `pictura-app` (`filter_tools/smart_apply.rs` and hooks, menus), a new Qt Test
  suite.
- `docs/05-layers/smart-filters.md`, `docs/dev/doc-code-parity-gaps.md`,
  `ROADMAP.md`.
