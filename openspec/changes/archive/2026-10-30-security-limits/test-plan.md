# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ui_png_profile_bomb` | Probe a PNG whose `iCCP` data inflates to 64 MiB | Present, no bytes; at most 16 MiB + 1 inflated | `pictura-codec::a_png_profile_bomb_is_refused` |
| `ui_png_profile_truncated` | Probe an `iCCP` chunk ending at the keyword NUL; probe a well-formed sRGB `iCCP` | First is present with no bytes, no panic; second carries the profile | `pictura-codec::a_truncated_png_profile_chunk_does_not_panic`, `pictura-codec::a_png_profile_within_the_limit_is_read` |
| `ui_gs_max_bitmap` | Build `png_args` at two resolutions | Both contain `-dMaxBitmap=536870912` before `-f` | `pictura-codec::render_arguments_band_large_pages` |
| `ac_path_refused` | Dispatch `document` `open` with `rel.psd`, `/tmp/a\nb.psd`, `<tmp>/x/../y.psd` | Each `invalid_param`; document count unchanged | `tst_untrusted_input::controlRefusesAmbiguousDocumentPaths` |
| `ac_path_save_link` | Plant a symlink to a victim file; `save_as` on the link | `invalid_param`; victim bytes unchanged | `tst_untrusted_input::controlRefusesASaveOverASymlink` |
| `ac_path_save_active_link` | Save a document, link to it, open the link, `save` with no path | `invalid_param`; target bytes unchanged; link kept | `tst_untrusted_input::controlRefusesAPathlessSaveThroughAnOpenedSymlink` |
| `ac_path_accepted` | `save_as` a new `<tmp>/plain.psd`, then `open` it | Both ok; file exists | `tst_untrusted_input::controlAcceptsAPlainAbsolutePath` |
