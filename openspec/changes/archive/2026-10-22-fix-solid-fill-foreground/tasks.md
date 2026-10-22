# Tasks

> The build and the test run were deferred for the first pass, so the code was
> written and reviewed at the source level first. Group 4 then ran the build,
> the suites, and the gates. The two foreground cases were proved red against
> the pre-fix colours before the fix was restored.

## 1. Regression tests (written first, red against the current code)

- [x] 1.1 In `crates/pictura-app/cpp/tests/tst_layers_panel_fixes.cpp`, add
  `#include "panels/color_panel.h"` — `frame.h` only forward-declares
  `ColorState`, so the type is incomplete without it — and add
  `solidFillUsesForeground`: open a document, find the shared colour state
  with `window_->findChild<pictura::ColorState*>()`, bind the view with
  `panel_->setView(view_)` and `panel_->refresh()` (the frame rebinds the
  panels through a debounced timer), then trigger the `Solid Color…` action
  found in `panel_->findChild<QToolButton*>("layersStripFillAdjustment")
  ->menu()`. Assert the new layer composites to the foreground colour.
- [x] 1.2 In the same file, add `solidFillUsesForegroundNotBackground`: set the
  foreground to black and the background to white, trigger the same
  `Solid Color…` action, and assert the new layer composites to black, not to
  the background. This one is a pin, not a red test: it passes against the
  pre-fix code, which also filled black, and it fails a fill that read the
  background.
- [x] 1.3 In `crates/pictura-app/cpp/tests/tst_layer_commands.cpp`, add
  `#include "panels/color_panel.h"` and add `solidFillCommandUsesForeground`:
  open a document, set the shared colour state's foreground to a non-black
  colour, assert the `LayerNewFillSolidColor` action is enabled, dispatch it
  through `registry->dispatch(id)`, and assert the new layer composites to
  that colour.

## 2. The panel reads the shared foreground

- [x] 2.1 In `crates/pictura-app/cpp/panels/layers_panel.h`, forward-declare
  `ColorState` and change the constructor to
  `explicit LayersPanel(ColorState* colors = nullptr, QWidget* parent = nullptr)`,
  then add the `colors_` member. The default keeps the two bare constructions
  — `selftest.cpp`'s `new pictura::LayersPanel()` and
  `tst_layers_panel_fixes.cpp`'s stack-allocated `pictura::LayersPanel panel;`
  — compiling unchanged.
- [x] 2.2 In `crates/pictura-app/cpp/panels/layers_panel.cpp`, store the state
  in the constructor and make the `Solid Color…` action pass the state's
  foreground colour instead of `0xff000000`; drop the "opaque black" ponytail
  comment above it.
- [x] 2.3 In `crates/pictura-app/cpp/frame_build.cpp`, pass `colorState_` to
  the panel where it is constructed.

## 3. The Layer menu path and the frame's foreground copy

- [x] 3.1 In `crates/pictura-app/cpp/frame.h`, drop the `foreground_` member
  and turn `foregroundColor()` into a declaration.
- [x] 3.2 In `crates/pictura-app/cpp/frame_build.cpp`, define
  `foregroundColor()` to return the shared colour state's foreground, drop the
  eyedropper's write to the frame's own copy, and read the accessor in the
  status-bar hint. Connect `ColorState::foregroundChanged` to
  `updateToolHint()` in `buildTools`, next to the other `ColorState` connects,
  so the hint follows the colour.
- [x] 3.3 In `crates/pictura-app/cpp/frame_menus.cpp`, keep the
  `LayerNewFillSolidColor` handler reading `foregroundColor()` and replace the
  comment above it, which still claims the colour "is not yet plumbed from the
  toolbox swatch" and "resolves to opaque black".
- [x] 3.4 In `crates/pictura-render/src/document_ops/layer_ops/create.rs`,
  correct the `ponytail:` comment on the dropped alpha byte: the callers may
  now pass a translucent foreground's alpha. Comment only, no behaviour.

## 4. Verification

- [x] 4.1 Build the app with CMake/Ninja.
- [x] 4.2 Run
  `ctest --test-dir build -R '^tst_layers_panel_fixes$|^tst_layer_commands$' --output-on-failure`;
  all three new cases must pass, and `solidFillUsesForeground` and
  `solidFillCommandUsesForeground` must fail when the two `add_solid_fill`
  call sites are put back to their old colours.
- [x] 4.3 Run
  `ctest --test-dir build -R '^tst_layers_panel$|^tst_command_tree$|^tst_swatches_panel$|^tst_fill_tools$' --output-on-failure`
  for the suites that touch the panel, the command registry, the colour state,
  and the fill tools.
- [x] 4.4 Run `./build/pictura --headless --self-test` and confirm the
  alt-eyedropper check still passes — it reads `foregroundColor()`.
- [x] 4.5 Run `python3 scripts/check-test-plan.py` and
  `openspec validate fix-solid-fill-foreground --strict`.
- [x] 4.6 Run `bash scripts/verify-fast.sh`.
- [x] 4.7 Archive the change with
  `openspec archive fix-solid-fill-foreground -y`.
