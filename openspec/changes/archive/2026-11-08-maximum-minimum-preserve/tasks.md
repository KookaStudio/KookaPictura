# Tasks

## 1. Spec and test plan

- [x] 1.1 Write the `imaging/other-filters` MODIFIED delta for the larger radius and the two footprints
- [x] 1.2 Write the `imaging/filter-app-ui` ADDED delta for the dialog, warning and commit
- [x] 1.3 Pair them with `test-plan.md`, one row per scenario
- [x] 1.4 Add the named tests before the implementation they cover

## 2. Engine

- [x] 2.1 Add `MorphologyPreserve` to `pictura-filters` and export it
- [x] 2.2 Add the `preserve` field to `Filter::Maximum` and `Filter::Minimum`, and update every constructor
- [x] 2.3 Add `disk_dilate` and `disk_erode` to `pictura-core::mask_ops` with the segment decomposition
- [x] 2.4 Raise `MAX_RADIUS` to 500 and dispatch the two footprints from `other::maximum`/`minimum`
- [x] 2.5 Add the `Preserve` choice to both catalogue rows and read slot 1 in the builders
- [x] 2.6 Decline `Roundness` in the GPU plan and lift the square radius cap

## 3. Dialog

- [x] 3.1 Add `FilterSpec::warning` and the `filter_param_warning` bridge
- [x] 3.2 Add the hidden `filterWarning` label to the shared filter dialog and update it on change and prefill
- [x] 3.3 Add `tst_maximum_filter` and register it in the tests `CMakeLists.txt`

## 4. Docs and bookkeeping

- [x] 4.1 Update `docs/06-filters/other-filters.md` and `public_docs/filter-video-other-digimarc.md`
- [x] 4.2 Tick the feature in `ROADMAP.md`, update `docs/dev/STATE.md`, and drop #364 from the parity-gap list
- [x] 4.3 Run `openspec validate --all --strict` and `python3 scripts/check-test-plan.py`
- [x] 4.4 Build once, run the affected Rust and Qt tests, and run `verify-fast.sh`
- [x] 4.5 Archive the change with `openspec archive maximum-minimum-preserve`
