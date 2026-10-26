# Tasks: smart-filter-masks

## Tests

- [x] Rust: every `pictura-codec` and `pictura-render` test named in `test-plan.md`

## Engine

- [x] `pictura-core`: `FilterMask` and `SmartObject::filter_mask`
- [x] `pictura-codec`: parse `FEid`/`FMsk` from `layer_section_extra` (ZIP-with-prediction channel), attach by uuid, `remove_filter_mask`
- [x] `pictura-render`: blend the chain through the mask in `composite_smart_filtered_source`

## App

- [x] `pictura-app`: `layer_has_smart_filter_mask`, `delete_layer_smart_filter_mask` (drop blocks, clear the typed mask, one state)

## Close-out

- [x] `docs/dev/doc-code-parity-gaps.md` if a gap is closed
- [x] `openspec validate --all --strict`, `scripts/check-test-plan.py`, then archive the change
