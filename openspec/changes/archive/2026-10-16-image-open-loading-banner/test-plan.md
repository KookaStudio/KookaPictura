# Test plan

One row per spec scenario in
`specs/document/document-lifecycle/spec.md`. Coverage names the automated test,
or `manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `olb_banner` | Open a PSD, a PNG, and an Open As Smart Object PNG; capture the banner text and visibility from `documentOpenStarted` | `Opening <file name>…` visible with a wait cursor during each load | `tst_open_loading_banner::bannerShownForEachOpenKind` |
| `olb_style` | Read the `#documentOpenBanner` rule from the built stylesheet | Rule sets the notice background, the dark notice text, and a border radius | `tst_design_system::openBannerUsesTheNoticeStyle` |
| `olb_clears` | Open a valid file, then a missing/undecodable file; inspect the banner and cursor after each returns | Banner hidden and cursor restored in both cases | `tst_open_loading_banner::bannerClearsOnSuccessAndRefusal` |
| `olb_notice` | Open a minimal flat CMYK PSD through the frame | Banner hidden; status bar shows `Converted from CMYK; saved as CMYK` | `tst_open_loading_banner::noticeReplacesBanner` |
