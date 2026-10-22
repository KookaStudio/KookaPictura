# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ui_psb_header` | Read a 300000² PSB with a tiny ZIP composite | Typed error; peak request under 4 MiB | `pictura-codec::a_psb_header_past_the_budget_is_refused_before_allocating` |
| `ui_composite_total` | Read a 56-channel 30000² PSD under a 16 GiB document limit | Typed error before the image data | `pictura-codec::a_composite_over_the_document_total_is_refused`, `pictura-codec::the_document_total_is_bounded` |
| `ui_layer_rect` | Read layers 2³¹ wide (RLE, ZIP, mask, depth 1) and an `i32`-spanning rect | Typed error; peak under 4 MiB; extents saturate | `pictura-codec::a_huge_rle_layer_rect_is_refused`, `pictura-codec::a_huge_zip_layer_rect_is_refused`, `pictura-codec::a_huge_mask_rect_is_refused`, `pictura-codec::a_huge_bitmap_layer_rect_is_refused`, `pictura-codec::a_rect_spanning_the_whole_i32_range_is_refused`, `pictura-core::psd_rect_extent_saturates_instead_of_overflowing` |
| `ui_document_total` | Read 8 and 2 layers of 200² under a 64 KiB plane and 512 KiB document limit | 8 fails, 2 opens | `pictura-codec::layer_planes_count_against_the_document_total`, `pictura-codec::one_allocation_over_the_cap_is_refused`, `pictura-codec::an_overflowing_product_is_refused` |
| `ui_canvas_opens` | Read a layered 16000² RGB PSD with no merged image under the default budget | Opens | `pictura-codec::a_canvas_class_document_header_still_opens` |
| `ui_host_budget` | Compute the limit for 48, 5, 1 GiB and unknown available; parse `/proc/meminfo`; read the host | 36 GiB, 3.75 GiB, 2 GiB, 16 GiB; bytes; the host reports memory | `pictura-codec::the_document_limit_follows_available_memory`, `pictura-codec::meminfo_available_is_parsed_in_bytes`, `pictura-codec::the_host_reports_available_memory` |
| `ui_document_limit` | Read a 16 MiB plane under an 8 MiB document limit | Refused; default at least 2 GiB | `pictura-codec::a_document_over_its_limit_is_refused_and_the_default_keeps_a_plane` |
| macOS and Windows readers | Open a large PSD on a macOS and a Windows host with plenty of free memory, then on one under memory pressure | Opens on the free host; refused cleanly (no crash) when the document exceeds three quarters of available memory | manual: only the Linux reader runs in this repo's test environment; the other platforms' readers are compile-checked by their release CI |
