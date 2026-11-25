# Test plan: tile-aware-brush-memory

Engine behaviour is checked by Rust unit tests in `pictura-paint`. The gate and
the measured heap peaks are checked in `pictura-app`. The heap guards need one
test per process and run under `cargo nextest`. The reproduction at full size
in the running app is the one manual row.

| Scenario | Steps | Expected result | Coverage |
| --- | --- | --- | --- |
| `bsm_undo_tiles` | Paint one dab in the middle of a 1024² textured layer and count the undo copy's tiles and bytes. | Exactly the 64² tiles the dab's saved box crosses are held, and the bytes are those tiles' bytes, far below the layer's. | `pictura-paint::one_dab_saves_only_the_tiles_under_it` |
| `bsm_cancel` | Paint an 8-bit brush and dodge across a tiled layer with partial edge tiles, and a 16-bit tone stroke, then cancel each. | The document equals its pre-stroke state, the 16-bit store included. | `pictura-paint::cancelling_on_a_tiled_layer_restores_partial_edge_tiles`, `pictura-paint::cancelling_a_stroke_restores_every_pixel_it_touched`, `pictura-paint::cancelling_a_depth16_tone_restores_the_native_store`, `pictura-paint::a_depth16_tone_grows_and_cancel_restores_the_native_store` |
| `bsm_grow` | Paint inside a textured layer smaller than the canvas, record the saved tiles, paint outside so the layer grows, then cancel. | The tiles saved before the grow remain and only the new dabs' tiles are added; cancel restores the original pixels and bounds. | `pictura-paint::a_grow_keeps_the_saved_tiles_and_cancel_restores`, `pictura-paint::paint_outside_rect_grows_layer` |
| `bsm_gate_sparse` | Pin available memory so the room is about 1.9 GiB, build a 21,000² document with a sparse 15,000 × 17,500 layer, and gate a stroke on it. | Admitted, while the old logical-size charge exceeds the room. | `pictura-app::a_sparse_layer_on_a_large_canvas_admits_a_stroke`; manual: the full-size reproduction in the running app (a 21k² canvas, a scribble layer, Memory Usage 10 %) paints without the memory message |
| `bsm_gate_dense` | Pin the room to 16 MiB, below a dense 2048² layer's 16 MiB of planes plus its 4 MiB canvas term, and gate a stroke on it and on an empty layer of the same canvas. | The dense layer is refused with the memory reason; the empty one is admitted. | `pictura-app::a_dense_layer_beyond_the_room_refuses_a_stroke` |
| `bsm_dense_peak` | Run the brush stroke on a dense 4250² layer alone in its process and compare the heap peak with `brush_bytes`. | Peak ≤ estimate, peak far below the layer's size, and within its recorded baseline. | `pictura-app::brush_stroke_dense_heap_peak`, `pictura-app::estimates_cover_the_measured_peaks` |
