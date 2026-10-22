# Tasks

## Tests

- [x] Qt: new `tst_type_placeholder` suite with every case in `test-plan.md`, registered in `cpp/tests/CMakeLists.txt`
- [x] Qt: `tst_warp_text::optionsButtonHasWarpTextIcon`

## Implementation

- [x] `options.warpText` icon: SVG, `lucide-map.json`, `pictura.qrc`; the type options-bar button uses it
- [x] `ToolController` placeholder flag shared with the type handlers; `beginSession` fills and selects it for new non-mask sessions
- [x] Preferences Type page, `edit.preferences.type`, the search index page list, session persistence, and frame wiring

## Close-out

- [x] `docs/dev/STATE.md`
- [x] `openspec validate --all --strict`, `scripts/check-test-plan.py`, then archive the change
