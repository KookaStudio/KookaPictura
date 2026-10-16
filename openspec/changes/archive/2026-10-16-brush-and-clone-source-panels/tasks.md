# Tasks: brush-and-clone-source-panels

## 1. Engine

- [x] 1.1 `SourceTransform` / `StampSource::transformed` (scale, flip, rotate about the anchor; bilinear); unit test.

## 2. Bridge

- [x] 2.1 `PaintTip` shared by every paint-tool begin; `begin_clone_stamp` takes the anchor and transform; `brush_tip_preview`.

## 3. App

- [x] 3.1 Controller Brush Tip Shape fields and five Clone Source slots; the Brush / Pencil and paint tools read the tip.
- [x] 3.2 `BrushPanel` and `CloneSourcePanel`; `Window > Panels` entries (Brush on F5); panel menus.
- [x] 3.3 Clone Stamp bar toggles for both panels.

## 4. Verification

- [x] 4.1 `brush_panel` (545) and `clone_source_panel` (546) self-tests; `bash scripts/verify-fast.sh`.
