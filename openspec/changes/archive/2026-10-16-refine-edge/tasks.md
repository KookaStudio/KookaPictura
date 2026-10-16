# Tasks: refine-edge

## 1. Engine (`pictura-select`)

- [x] 1.1 `refine.rs`: `RefineEdgeSettings`, `ViewMode`, `OutputTarget`, `refine`.
- [x] 1.2 Edge band (`edge_band`), smart-radius adaptation, coverage re-estimation.
- [x] 1.3 Global refinements: contrast and shift-edge on top of the reused smooth/feather.
- [x] 1.4 `decontaminate(image, alpha, amount) -> PixelBuffer`.

## 2. App (`pictura-app`)

- [x] 2.1 `cxxqt_object/refine.rs` bridge: available, preview, apply, decontaminate.
- [x] 2.2 `RefineEdgeDialog` (`refine_edge_dialog.{h,cpp}`) with the CS6 layout.
- [x] 2.3 Command wiring: `Select ▸ Refine Edge…` (`Ctrl+Alt+R`) and the options-bar `Select and Mask…` button.

## 3. Verification

- [x] 3.1 Rust property tests: identity at defaults, radius bounds change, contrast bimodal, shift direction, smooth removes specks, decontaminate moves fringe colour.
- [x] 3.2 `tst_refine_edge`: opens, previews, OK records one state, Cancel is byte-identical, decontaminate disables in-place output.
- [x] 3.3 `bash scripts/verify-full.sh`; `openspec validate --all --strict`.
