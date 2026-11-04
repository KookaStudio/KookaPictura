# Tasks

## 1. Tests

- [x] 1.1 Add the tests in `test-plan.md`

## 2. Measurement

- [x] 2.1 Give `profile-tools.py` a scenario for every toolbox tool
- [x] 2.2 Profile every tool at 17000×17000 and record the baseline

## 3. Lassos

- [x] 3.1 Fill the lasso polygon by scanline within its rows; report its bounds
- [x] 3.2 Feather and combine a lasso or marquee only within its box
- [x] 3.3 Seed the selection-bounds cache from a lasso's New or Add commit
- [x] 3.4 Find the Magnetic Lasso peak in one pass; compute its tiles on read

## 4. Shapes

- [x] 4.1 Bound a fill or shape layer's composite, effects source and GPU fill to its mask's box
- [x] 4.2 Render the shape Move preview over its paint bounds and patch its influence
- [x] 4.3 Commit a shape Move over its influence before and after
- [x] 4.4 Composite a path drag on a shape layer on its release, over its influence before and after
- [x] 4.5 Rasterize and fill a Fill Pixels shape in its own box, in the bands it crosses

## 5. Evidence

- [x] 5.1 Time the stages in `shape_lasso_profile`
- [x] 5.2 Re-profile at 17000×17000 and compare (a real display stays the manual `slr_large_manual` row)
