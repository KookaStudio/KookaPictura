# Tasks

## 1. Engine

- [x] 1.1 `blur` dispatch in `crates/pictura-select/src/lib.rs`: direct Gaussian below `sigma = 2`, variance-matched three-box stack at or above.
- [x] 1.2 Unit tests: step within 4 and disk within 6 coverage levels of the direct Gaussian at radius 4/8/20/100/250; solid and empty planes unchanged.
- [x] 1.3 Ignored `feather_selection_profile` records the numbers in `design.md`.

## 2. Verification

- [x] 2.1 `cargo nextest run -p pictura-select`; the ImageMagick oracle still passes byte-exact at radius 2.
- [x] 2.2 `bash scripts/verify-full.sh`; `openspec validate --all --strict`.
