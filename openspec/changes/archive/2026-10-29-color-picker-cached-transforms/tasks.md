# Tasks

## 1. Test plan

- [x] 1.1 Write `test-plan.md` with one row per spec scenario

## 2. Reusable cross-space transform (`pictura-color`)

- [x] 2.1 Add `SpaceTransform` and rebuild `convert_space` on it; add `space_transform_matches_convert_space`

## 3. Picker transform cache (`pictura-app`)

- [x] 3.1 Replace the stored ICC bytes with the cached `PickerTransforms` pair, route the four picker services through it, and have `set_color_settings` install the pair
- [x] 3.2 Add `picker_services_reuse_their_transforms` and `installing_a_profile_rebuilds_the_picker_pair`

## 4. GUI check

- [x] 4.1 Add `tst_color_picker::fieldDragUpdatesEverySample`

## 5. Verify

- [x] 5.1 Build, then run the touched suites, `openspec validate --all --strict`, and `bash scripts/verify-fast.sh`
