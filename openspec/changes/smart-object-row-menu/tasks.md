# Tasks

## 1. Row table

- [x] 1.1 Add `convertToSmartObject` (Pixel) and `editContents` / `replaceContents` / `exportContents` / `relinkToFile` / `convertToLinked` (SmartObject) rows to `kRowSpecs` in `panels/layers_panel_menu.cpp`, with `ConvertToSmartCan` / `EditContentsCan` / `ReplaceContentsCan` / `ExportContentsCan` enablement calling the existing view predicates; render the two linked rows disabled with the not-implemented tooltip
- [x] 1.2 Dispatch `convertToSmartObject` through `view_->convert_to_smart_object` (refresh + select), and emit a new `smartObjectContentsRequested(action, path)` signal for `editContents` / `replaceContents` / `exportContents`; declare the signal in `panels/layers_panel.h`

## 2. Frame wiring

- [x] 2.1 Add `replaceSmartObjectContents(path)` and `exportSmartObjectContents(path)` to `PicturaMainWindow` (declared in `frame.h`, defined in `frame_menus_smart_object.cpp`) by extracting the existing Replace/Export dialog logic, and make the `Layer > Smart Objects` command handlers delegate to them
- [x] 2.2 Add `wireSmartObjectRowMenu()` (defined in `frame_menus_smart_object.cpp`, called from `frame_menus.cpp`) that connects `LayersPanel::smartObjectContentsRequested` to Edit / Replace / Export on the row path

## 3. Tests

- [x] 3.1 Update `tst_command_tree.cpp` expected pixel row list for the new Convert to Smart Object row
- [x] 3.2 Extend `tst_smart_object_actions.cpp`: the smart row offers the contents rows and the disabled linked rows; Convert to Smart Object from the row menu records one state and selects the object; a `QSignalSpy` verifies the contents rows emit on the row path

## 4. Verification

- [x] 4.1 `cmake --build build --parallel`, then `ctest --test-dir build -R '^tst_command_tree$|^tst_smart' --output-on-failure`
- [x] 4.2 `bash scripts/verify-fast.sh` green and `openspec validate --all --strict`
