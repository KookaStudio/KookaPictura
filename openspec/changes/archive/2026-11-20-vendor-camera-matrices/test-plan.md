# Test plan

This plan covers `vendor-camera-matrices` (#1465): the new capability
`codec/camera-matrices` and the modified `codec/vendor-raw-containers`.

**Fixtures.** Synthetic CR2s are written in the test by
`tests/camera_raw/raw/cr2_builder.rs`, which gains maker-note colour data (tag `0x4001`)
and IFD0 make/model; planes are deterministic (fixed LCG seed). The corpus row
reads user-supplied camera files, opt-in through `PICTURA_RAW_CORPUS`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `camera_matrices_lookup` | CR2s with make "Canon" and models 5DS, 5DS R, 400D DIGITAL, DIGITAL REBEL XTi, Kiss Digital X, EOS-1Ds Mark III | Both matrices, illuminants [17, 21]; 5DS = 5DS R; the 400D names agree and differ from 5DS | `pictura-codec::known_canon_models_take_fitted_matrices` |
| `camera_matrices_unknown` | CR2s for "Canon EOS 450D", make "Nikon" + model "EOS 5DS", no make/model; decode the 450D | No matrices; 450D refused with "camera colour" and "Canon EOS 450D" | `pictura-codec::unknown_cameras_keep_no_matrices` |
| `camera_matrices_white` | For each table row: build `DngColor`, set `AsShotNeutral` to the D65 (then A) matrix's response to its white; `as_shot_transform` | Neutral positive; 6504 K / 2856 K within 2 % | `pictura-codec::fitted_matrices_keep_white_neutral` |
| `camera_matrices_generated` | Clone rawtoaces-data at the pinned commit, download the CIE D65 CSV, run `python3 scripts/gen-camera-matrices.py <data> <csv>`, `git diff` | No diff | manual: needs the external datasets, which are not vendored |
| `cr2_as_shot_layouts` | CR2s with colour data of 582/653 entries and versions 1, 6, 10, 15, 17, -3, 33, 48; multipliers (2000, 1000, 1024, 1500) at the layout's index | `AsShotNeutral` (1012/2000, 1, 1012/1500) within 1e-9 | `pictura-codec::cr2_as_shot_neutral_by_layout` |
| `cr2_as_shot_refused` | Version 25; version 10 with 40 entries; version 10 with red 0 | Reads; no `AsShotNeutral` | `pictura-codec::unusable_colour_data_leaves_no_neutral` |
| `cr2_known_camera_decodes` | "Canon EOS 5DS" CR2, version 12, multipliers (2208, 1024, 1024, 1967); `decode_raw_source` | Ok; as-shot = `as_shot_transform(color)`, 2000–12000 K | `pictura-codec::known_camera_cr2_decodes` |
| `cr2_colour_corpus` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test camera_raw raw_camera_matrices::corpus` | Table cameras decode with 2000–12000 K; others refused (checked on 16 files: 13 table bodies decode; 450D, 350D and 600D refused) | `pictura-codec::cr2_corpus_colour_when_present` |
| `cr2_levels` | Unchanged | Unchanged | `pictura-codec::masked_border_sets_the_black` |
| `cr2_tags` | Unchanged (model "Canon EOS Synthetic", no colour data) | Colour tags empty | `pictura-codec::cr2_reports_camera_tags` |
| `cr2_hostile` | Unchanged | Unchanged | `pictura-codec::hostile_cr2s_fail_cleanly` |
| `cr2_corpus` | Unchanged | Unchanged | `pictura-codec::cr2_corpus_reads_when_present` |
