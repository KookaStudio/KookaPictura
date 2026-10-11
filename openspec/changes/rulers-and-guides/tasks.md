# Tasks: rulers-and-guides

## 1. Core and codec

- [x] 1.1 `pictura-core`: `Guide`, `GuideOrientation`, `guide_near`, `Document.guides`.
- [x] 1.2 `guide_resources.rs`: read 1032 into `guides`; write it back, verbatim when unchanged.
- [x] 1.3 Unit tests plus the `guide_resource_oracle` psd-tools oracle.

## 2. App

- [x] 2.1 `cxxqt_object/guides.rs` bridge: count/at/near/add/move/remove/clear, with history labels.
- [x] 2.2 `CanvasRuler` and the canvas host's ruler row/column; guide drop from a ruler.
- [x] 2.3 `ImageView` guide overlay, preview, colour and style.
- [x] 2.4 `ToolController` guide drags (Move tool or Ctrl), Move-tool hover cursor.
- [x] 2.5 View menu: Rulers, Show > Guides, Lock Guides, Clear Guides, New Guide….
- [x] 2.6 Guides, Grid, & Slices preferences page and session persistence.
- [x] 2.7 Ruler units (context menu) and the Units & Rulers preferences page (#299).

## 3. Verification

- [x] 3.1 Qt Test `tst_rulers_guides`; updated the `prefs_pages` self-test expectation for the new real page.
- [x] 3.2 `bash scripts/verify-fast.sh` and the CMake build.
