# Tasks

## 1. Tests

- [x] 1.1 Add `tst_layers_panel_bundle::shortLayerNameIsNotElided`: a fitting
  name (`r1 r2`) paints in full, the Background, group, and styled rows keep
  their full names, and an overlong name middle-elides once keeping both ends.
- [x] 1.2 Add `LayersPanel::rowNameTextForTest` so the test reads the exact
  string the delegate paints.

## 2. App

- [x] 2.1 Have `LayerRowDelegate::nameRect` report its elided text through an
  optional out-parameter.
- [x] 2.2 Draw the regular and synthetic row names from that text instead of
  re-eliding to `nameRect`'s width.
- [x] 2.3 Elide in the middle (`Qt::ElideMiddle`) so both ends of a long name
  stay visible.

## 3. Gates

- [x] 3.1 Build and run `tst_layers_panel_bundle`.
- [x] 3.2 `python3 scripts/check-test-plan.py`, `openspec validate --all --strict`.
- [x] 3.3 Archive with `openspec archive layer-name-elision`.
