# Test plan

One row per scenario in the delta spec. Every row is automated.

## document/document-lifecycle

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ndb_swatch_follows_contents` | Open the New Document dialog and inspect Background Contents and the swatch; choose Background Color; choose Transparent; choose White again. | The list is White, Background Color, Transparent (no Black or Custom); the swatch shows white, then the constructor's toolbox background, stays enabled for the color choices, and is disabled for Transparent. | `tst_new_document::backgroundSwatchFollowsBackgroundContents` |
| `ndb_swatch_pick` | Click the swatch from White, type `336699` in the picker's hex field, accept it, and build a document from the dialog's spec. | Custom is added and selected, the swatch shows `#336699`, the spec's fill is `#336699`, and the document's locked Background composite is that color. | `tst_new_document::backgroundSwatchPicksAndCancelsACustomColor` |
| `ndb_swatch_cancel` | Click the swatch while Custom is selected and reject the picker. | Custom stays selected, and the swatch and spec fill are unchanged. | `tst_new_document::backgroundSwatchPicksAndCancelsACustomColor` |
| `ndb_swatch_transparent` | Choose Transparent and read the dialog's spec. | The swatch is disabled and the spec requests a transparent document. | `tst_new_document::backgroundSwatchFollowsBackgroundContents` |
