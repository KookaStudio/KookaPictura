# Tasks

## 1. Model and codec

- [x] 1.1 `LayerMask` / `VectorMask` gain `density` and feather bits with accessors.
- [x] 1.2 Parse the parameters block (real-mask header keyed on `-3`), route the vector half, drop parameter-only records.
- [x] 1.3 Write the block back (spliced after the real-mask header, padded to 4); parameter-only record for vector-only values.
- [x] 1.4 Unit round-trip tests; psd-tools oracle reads back the written `MaskParameters`.

## 2. Compositing and edits

- [x] 2.1 Density and feather in `mask_alpha` / `mask_alpha_unit`; cached feathered planes; widened influence rect.
- [x] 2.2 `set_{layer,vector}_mask_{density,feather}` and `invert_layer_mask`, one undo step each.
- [x] 2.3 Unit tests for density 0/50/100, feather ramp, invert, and undo.

## 3. UI

- [x] 3.1 Bridge functions in `impl_layers/layer_masks.rs` and `vector_masks.rs`.
- [x] 3.2 Properties Density / Feather / Invert controls; Qt Test cases.

## 4. Verification

- [x] 4.1 `bash scripts/verify-full.sh`; `openspec validate --all --strict`; `STATE.md`.
