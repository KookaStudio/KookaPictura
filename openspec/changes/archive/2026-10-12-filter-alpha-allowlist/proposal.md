# Proposal: filter-alpha-allowlist

## Why

Issue #248. On an unlocked layer, `apply_op_region` reruns the filter over a
grey copy of the alpha plane and writes the result back as transparency. That
is only right for a kernel that maps a flat 255 plane back to 255. A
blocklist (`filter_preserves_opacity`) exempted four kinds; every other
value-dependent or plane-replacing kernel rewrote opacity: Solarize turned an
opaque layer fully transparent, Clouds left the cloud tone as alpha, and Offset
without wrap cleared the whole layer with a black Background.

## What Changes

- `filter_preserves_opacity` becomes an allowlist: only kernels that move,
  spread, or rank samples independently of their value (blurs, sharpens, rank
  filters, Offset, Mosaic, Crystallize, Fragment, the geometric Distort family)
  run the alpha pass. Every other kind leaves alpha bit-identical.
- Offset's alpha pass fills the exposed area with 255: the dialog's single
  `Background:` colour is CS6's "Set to Background", which paints opaque on a
  normal layer.
- **Follow-up (tasks 3.1/3.2):** the grey alpha pass is gone — alpha is folded
  into the filter's working buffer as a fourth plane and each kernel decides
  it (moving kernels process it, value kernels cap at three planes), across
  the native kernels, the photorust write-back, and the GPU plane count. The
  Offset dialog gains CS6's "Set to Transparent" alongside "Set to Background".
- The `filter-application` alpha requirement, which still read "MUST NOT be
  modified", states the actual contract, including both Offset fill modes.

## Capabilities

### Modified Capabilities

- `imaging/filter-application`: the alpha requirement names which kinds filter
  an unlocked layer's transparency and which leave it bit-identical, and pins
  both Offset fill modes.
- `imaging/other-filters`: the Offset requirement gains `transparent` and the
  two exposed-area fill rules (tasks 3.1/3.2).

## Impact

`crates/pictura-render/src/filter.rs` (fold, grey pass deleted) plus, for the
follow-up tasks: every moving kernel in `crates/pictura-filters` (native plane
caps, photorust alpha write-back, `filters_alpha` predicate), the GPU filter
path in `crates/pictura-render/src/gpu_filter/` (plane count from
`filters_alpha`), `Filter::Offset`'s new `transparent` field and its dialog
slot in `crates/pictura-app/cpp/filter_commands.cpp` / `filter_map.rs`
(arity 6 → 7; params still cross the existing `QList<f64>` bridge, so no
bridge signature change). No new dependencies.
