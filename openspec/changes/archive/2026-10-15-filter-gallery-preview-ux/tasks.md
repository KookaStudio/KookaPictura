# Tasks

## 1. Zoom control

- [x] 1.1 Replace the gallery's zoom label with an editable `QComboBox`
  (`galleryZoom`) showing the live percentage and listing CS6's presets
  (6 %–1600 %, separators, Actual Pixels / Fit in View / Fit on Screen); apply
  the selection on `activated` and a typed value on `editingFinished`
- [x] 1.2 Factor the +/− step logic into `stepZoom` and zoom on a mouse wheel
  over the preview viewport
- [x] 1.3 Add `zoomDropdownAndWheel` to `tst_filter_gallery.cpp` (preset list,
  wheel zooms in, a preset applies); verify `ctest --test-dir build -R tst_filter_gallery`

## 2. Thumbnails toggle

- [x] 2.1 Give the toggle the Lucide `panel-right-close` / `panel-right-open`
  icon by state and the shared flat button surface; add the two icons to
  `assets/icons`, `lucide-map.json`, and `pictura.qrc`; verify
  `python3 scripts/check-icon-provenance.py`

## 3. Slider activates its value box

- [x] 3.1 Focus the row's spin box on `sliderPressed` in `FilterParamControls`
- [x] 3.2 Add `sliderClickFocusesValueBox` to `tst_filter_gallery.cpp`; verify
  `ctest --test-dir build -R tst_filter_gallery`

## 4. Integration

- [x] 4.1 Run `cmake --build build --parallel`, `ctest --test-dir build -R '^tst_'`,
  `./build/pictura --headless --self-test`, and `bash scripts/verify-full.sh`;
  verify all green, then update `docs/dev/STATE.md`
- [x] 4.2 File a GitHub issue for the pan-performance improvement (issue #452)
