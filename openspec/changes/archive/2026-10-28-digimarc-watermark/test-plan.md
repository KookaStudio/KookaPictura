# Test plan — digimarc-watermark

One row per scenario in the change's spec deltas
(`specs/imaging/digimarc-watermark/spec.md` and
`specs/ui/application-shell/spec.md`). Coverage names the test that proves the
row; `manual:` is used only where the check cannot run in CI.

Tests named here are created by `tasks.md`; `scripts/check-test-plan.py` reports
only "no such test" until the implementation tasks land them.

Run rows with:

```bash
cargo nextest run -p pictura-filters digimarc
cargo nextest run -p pictura-filters properties
cargo nextest run -p pictura-app read_watermark
cargo nextest run -p pictura-script catalogue
ctest --test-dir build -R '^tst_digimarc_watermark$' --output-on-failure
```

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `dw_roundtrip` | Embed a payload at Low, Medium, and High durability, then read the pixels back | The reader returns the same type, creator ID, ID, and durability | `pictura-filters::watermark_round_trips_at_each_durability` |
| `dw_crc_reject` | Overwrite a watermarked image's pixels so the codeword no longer validates, then read | The reader returns no watermark rather than a wrong payload | `pictura-filters::corrupted_carrier_is_rejected` |
| `dw_absent` | Read an image that was never embedded | The reader returns no watermark | `pictura-filters::plain_image_has_no_watermark` |
| `dw_carrier` | Embed the same payload twice and compare; embed into a four-channel buffer and diff the alpha plane | The results are byte-identical, every colour sample moves by at most the amplitude, and alpha is unchanged | `pictura-filters::carrier_is_deterministic_and_alpha_safe` |
| `dw_durability` | Embed the same payload at Low and at High and compare the reported margins | The High margin is greater than the Low margin | `pictura-filters::higher_durability_has_a_larger_margin` |
| `dw_embed_menu` | Open a document, refresh the registry, look up the Embed Watermark action | The action exists, is enabled, and is labelled with an ellipsis | `tst_digimarc_watermark::embedMenuIsEnabledAndOpens` |
| `dw_embed_controls` | Build the Embed Watermark dialog and inspect its controls | It has a Watermark Type choice, Creator ID and Image ID number fields, and a Low/Medium/High durability radio group | `tst_digimarc_watermark::dialogExposesCs6Controls` |
| `dw_embed_commit` | Dispatch Embed Watermark, accept the dialog, and compare pixels and history; then cancel and compare again | Accepting changes pixels, records one `Filter: Embed Watermark` state, and the payload reads back; cancelling changes nothing | `tst_digimarc_watermark::commitRecordsOneStateAndCancelRestores` |
| `dw_read_menu` | Open a document with a pixel layer and look up the Read Watermark action | The action exists and is enabled | `tst_digimarc_watermark::readMenuIsEnabled` |
| `dw_read_found` | Embed a payload on the active layer, then read it through the bridge | The decoded row names the embedded type, creator ID, and ID | `tst_digimarc_watermark::readReportsEmbeddedPayload` |
| `dw_read_none` | Read a layer with no carrier through the bridge | The decoded row reports no watermark | `tst_digimarc_watermark::readReportsNoWatermark` |
| `dw_title` | Embed a payload, read it, and inspect the document label | The label carries a copyright symbol | `tst_digimarc_watermark::detectedWatermarkMarksTheTitle` |
| `dw_title_plain` | Inspect the label of a document that was never watermarked, and of a read that found nothing | Neither label carries a copyright symbol | `tst_digimarc_watermark::undetectedWatermarkLeavesTheTitlePlain` |
