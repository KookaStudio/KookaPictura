# Tasks: filter-alpha-allowlist

## 1. Engine

- [x] 1.1 Invert `filter_preserves_opacity` into an allowlist of moving kernels.
- [x] 1.2 Run Offset's alpha pass with a 255 background so "Set to Background" stays opaque.

## 2. Verification

- [x] 2.1 Value kernels leave alpha bit-identical on opaque and half-transparent layers.
- [x] 2.2 Moving kernels keep an opaque layer opaque.
- [x] 2.3 Offset moves the layer edge and fills the exposed area opaque.

## 3. Follow-ups

- [x] 3.1 Fold alpha into the working buffer so each kernel receives the fourth plane, and drop the grey pass.
- [x] 3.2 Offer CS6's "Set to Transparent" for Offset's undefined area.
