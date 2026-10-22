# Tasks

## 1. Test plan and test scaffold

- [x] 1.1 Write `test-plan.md` in this change folder mapping every spec scenario to an automated test or a justified manual check, and verify `scripts/check-test-plan.py` passes on the change
- [x] 1.2 Add `tst_adjustments_panel.cpp` to `crates/pictura-app/cpp/tests/CMakeLists.txt` and verify the build registers the suite
- [x] 1.3 Write `gridShowsAllNineteenInOrder` (19 buttons, fill-first order, icon-only with tooltips) and verify it fails before the widget exists
- [x] 1.4 Write `gridEntriesCarryTooltips` and verify it fails before the widget exists
- [x] 1.5 Write `clickAdjustmentCreatesAndSelectsLayer` (per adjustment kind: row count +1, layer selected, Properties shows controls) and verify it fails before the widget exists
- [x] 1.6 Write `adjustmentCreateIsUndoable` and verify it fails before the widget exists
- [x] 1.7 Write `solidColorIconDispatchesFillCommand` and `patternIconIsInert` and verify they fail before the widget exists
- [x] 1.8 Write `noDocumentDisablesGrid` and `documentEnablesGrid` and verify they fail before the widget exists
- [x] 1.9 Write `narrowPanelWrapsGrid` (shrink panel width, assert all entries visible) and verify it fails before the widget exists

## 2. Icon assets

- [x] 2.1 Add the 14 Lucide map entries (kind `lucide` with slugs: `circle-dot`, `grid-3x3`, `chart-column`, `chart-spline`, `sun-moon`, `palette`, `contrast`, `camera`, `sliders-horizontal`, `box`, `image-minus`, `rows-4`, `square-split-horizontal`, `pipette`) to `assets/icons/lucide-map.json` and verify `scripts/check-icon-provenance.py` reports only the expected missing files
- [x] 2.2 Vendor the 14 Lucide SVGs from the pinned v1.50.0 release with `scripts/sync-lucide-icons.py` and verify the files exist under `assets/icons/`
- [x] 2.3 Hand-author the 5 custom SVGs (`adjustments.brightness-contrast`, `adjustments.vibrance`, `adjustments.color-balance`, `adjustments.gradient`, `adjustments.gradient-map`, the gradient pair derived from `tool.gradient.svg`) to the Lucide style contract and verify the provenance guard passes on them
- [x] 2.4 Register all 19 icons in `assets/pictura.qrc` and verify a build embeds them (`icon(id)` non-null for each `adjustments.*` id)
- [x] 2.5 Update `docs/dev/icon-provenance.md` with the 19 entries and verify the provenance guard passes
- [x] 2.6 Copy the 19 SVGs to `site/icons/` and update the icon gallery in `site/index.html` to show them, verifying the page references resolve

## 3. AdjustmentsPanel widget

- [x] 3.1 Implement `panels/adjustments_panel.{h,cpp}` (static 19-row table, QGridLayout of QToolButtons, IconOnly, tooltips, dispatch via the existing `adjustment:<kind>` and fill-command paths) and verify the test suite from group 1 passes
- [x] 3.2 Add the new sources to the top-level `CMakeLists.txt` and verify the CMake configure picks them up
- [x] 3.3 Swap `PlaceholderPanel*` for `AdjustmentsPanel*` in `frame.h:760` and construct it in `frame_build.cpp:77` with the same `objectName` (`adjustmentsPanel`), verifying the panel registers and the Window menu still lists it
- [x] 3.4 After an adjustment icon's create dispatch, select the newly-created layer through the Layers-panel selection API and verify `clickAdjustmentCreatesAndSelectsLayer` passes end-to-end (layer selected, Properties shows controls)
- [x] 3.5 Wire document-open enablement into the frame's existing document-change notification and verify `noDocumentDisablesGrid`/`documentEnablesGrid` pass

## 4. Docs and roadmap

- [x] 4.1 Tick `ROADMAP.md:344` (#509) and update the panels placeholders count (line 33), verifying the roadmap still renders
- [x] 4.2 Flip the PAN-1 row in `docs/dev/doc-code-parity-gaps.md` (row ~line 99, evidence `frame_build.cpp:77`) from MISSING to the shipped state (docs commit carries `TASK-ALLOWS-DOCS`)

## 5. Integration verification

- [x] 5.1 Run `openspec validate --all --strict` and verify the change validates
- [x] 5.2 Build once and run the Qt layer offscreen (`ctest --test-dir build -R '^tst_' --output-on-failure`), verifying `tst_adjustments_panel` and `tst_command_tree::layerAdjustmentMenu` pass
- [x] 5.3 Run `scripts/check-icon-provenance.py` and `scripts/check-test-plan.py` and verify both pass
