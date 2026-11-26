# Tasks: knockout-oracle

## 1. Fixture

- [x] 1.1 Add a `knockout()` builder to `scripts/generate-fixtures.py`: 8x8 RGB, `PSDImage.new(..., color=(255,0,0))` background, a green `create_pixel_layer`, a blue `create_pixel_layer` with `opacity = 128`, and `blue._record.tagged_blocks.set_data(Tag.KNOCKOUT_SETTING, 2)` (Deep). Register it in `FIXTURES` as `knockout.psd`.
- [x] 1.2 Regenerate: `python3 scripts/generate-fixtures.py`; commit `crates/pictura-codec/tests/fixtures/knockout.psd`. Confirm a re-run produces byte-identical output.

## 2. Reference

- [x] 2.1 Add `scripts/psd_knockout_reference.py` that opens the fixture with `psd-tools`, calls `psd.composite()`, converts to `RGBA`, and writes raw interleaved RGBA8 to the render fixtures dir; a `gen` subcommand like `im_compose.py`.
- [x] 2.2 Run it; commit `crates/pictura-render/tests/fixtures/knockout_deep.rgba`.

## 3. Render test

- [x] 3.1 Add `crates/pictura-render/tests/oracle/knockout_oracle.rs`: `read_psd` the fixture, assert `Knockout::Deep`, `composite_rgba`, `pictura_testkit::compare` against the reference at the chosen tolerance, assert the green is punched through, and self-skip when `psd_tools` is absent.
- [x] 3.2 Record the tolerance and the psd-tools provenance in `crates/pictura-render/tests/README.md` (mirror the ImageMagick note).

## 4. Docs and gates

- [x] 4.1 Add the fixtures to `crates/pictura-codec/tests/fixtures/README.md` and note the psd-tools knockout oracle in the roadmap/STATE open-question text.
- [x] 4.2 `cargo nextest run -p pictura-render -p pictura-codec`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `openspec validate knockout-oracle --strict`.
- [x] 4.3 If the oracle disagrees beyond a 1-2 LSB tolerance, STOP and report the per-pixel diff rather than widening the tolerance.
