# Test plan

This plan covers `raw-settings-storage` (#401): the new capabilities
`codec/raw-settings-xmp` and `document/raw-settings-storage`, and the modified
seed requirement of `document/raw-open`.

**Fixtures.** Codec tests use inline XMP packets. The app's Rust tests reuse the
minimal uncompressed DNG writer in `raw_open/tests.rs` and pass an explicit
temporary database folder, so they never touch the user's data. The Qt suite
`tst_raw_open` points `XDG_DATA_HOME` at a temporary folder for the whole suite
(the existing OK test now writes a record too). Read-only rows make the folder
`0555` and skip when the process can still write there (running as root).

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `raw_xmp_reads_tabs` | Read a packet setting Exposure, Sharpness, orange hue, split-toning balance, post-crop vignette, camera profile, grayscale, process version, WhiteBalance Custom, and a 3-point RGB curve | Each control holds the stated value; WB `Cst `, PV 2012 | `pictura-codec::xmp_reads_controls_from_every_tab` |
| `raw_xmp_reads_nothing` | Read a packet with only `dc:title` | Default settings | `pictura-codec::xmp_without_crs_reads_default` |
| `raw_xmp_round_trip` | Write settings with every tab set (four curves, text with `&` and `<`) with no base, read back | Equal to the written settings | `pictura-codec::xmp_round_trips_every_tab` |
| `raw_xmp_preserves_unknown` | Rewrite a sidecar with an unknown crs key, a `crs:RetouchInfo` Seq and `dc:title` at exposure 0.25 | Output equals the input with only `+1.00` replaced by `+0.25` | `pictura-codec::xmp_rewrite_keeps_unknown_bytes` |
| `raw_xmp_removes_unset` | Rewrite a self-closing description with Contrast set, settings unset contrast and set an RGB curve | No `Contrast2012`; curve Seq inside the description; curve reads back | `pictura-codec::xmp_rewrite_removes_unset_and_inserts_curves` |
| `raw_xmp_refuses_unpatchable` | Write over `not xmp` | `None` | `pictura-codec::xmp_without_rdf_is_refused` |
| `raw_settings_key_sha256` | Key of `abc` | The published SHA-256 of `abc` | `pictura-codec::settings_key_is_sha256` |
| `raw_store_preference` | Pick Sidecar in the Preferences combo; new window from the saved session; fresh session | Coordinator store Sidecar; fresh store Database | `tst_raw_open::settingsStorePreferencePersists` |
| `raw_store_database_by_content` | Save exposure 0.75 to the database for `shot.dng`; decode the same bytes as `moved.dng` elsewhere | `<sha256>.xmp` exists, no sidecar, DNG bytes unchanged, moved seed exposure 0.75 | `pictura-app::database_record_follows_content` |
| `raw_store_ok_writes` | Store Sidecar; driver sets Exposure +1.00 and presses OK | `shot.xmp` holds `Exposure2012="+1.00"` and `WhiteBalance="As Shot"`; one document | `tst_raw_open::okWritesSidecar` |
| `raw_store_done_writes` | Store Database; driver presses Done | No document; database holds one record | `tst_raw_open::doneWritesDatabaseOnly` |
| `raw_store_cancel_writes_nothing` | Store Sidecar; one dialog Cancel, one non-interactive open | No sidecar; database folder empty | `tst_raw_open::cancelAndNonInteractiveWriteNothing` |
| `raw_store_keeps_seed_record` | Store Database; sidecar with exposure 1.0 and unknown key; save exposure 0.5 | Record exposure 0.5 plus unknown key; sidecar bytes unchanged | `pictura-app::database_record_keeps_sidecar_keys` |
| `raw_store_white_balance_label` | Save with the As Shot preset, as-shot tint, temperature +500 K | Record `WhiteBalance="Custom"` | `pictura-app::edited_white_balance_is_custom` |
| `raw_store_read_only_fallback` | Store Sidecar; folder `0555`; save | `DatabaseFallback`; no sidecar; database record exists | `pictura-app::read_only_folder_falls_back_to_database` |
| `raw_store_unwritable_warns` | Store Sidecar; folder `0555`; `XDG_DATA_HOME` a regular file; OK | Status message contains "not saved"; document opens | `tst_raw_open::unwritableStoresWarnAndOpen` |
| `raw_open_seed_as_shot` | Decode with no records | Seed temperature/tint = rounded as-shot | `pictura-app::seed_uses_as_shot_white_balance` |
| `raw_open_seed_sidecar` | Sidecar 3200 K / +1.00, embedded 4300 | 3200, 1.0 | `pictura-app::sidecar_overrides_seed` |
| `raw_open_seed_embedded` | Embedded 4300, no records | 4300 | `pictura-app::embedded_xmp_seeds_without_sidecar` |
| `raw_open_seed_preferred_store` | Sidecar +1.00, database −1.00; decode with each preference | −1.0 with Database, 1.0 with Sidecar | `pictura-app::preferred_store_is_read_first` |
| `raw_open_seed_all_tabs` | Sidecar Sharpness 55, SplitToningShadowHue 210 | Seed holds both | `pictura-app::record_seeds_every_tab` |
| `raw_open_seed_as_shot_label` | Sidecar WhiteBalance As Shot, Temperature 9000 | Seed temperature = rounded as-shot | `pictura-app::as_shot_record_keeps_as_shot_balance` |
