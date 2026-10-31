# Test plan — bound-retouch-stroke-start

Every scenario from `specs/tools/stroke-source-surfaces/spec.md`. The coverage
cell names the automated test; the apply phase must create each one.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `ss_layer_parity` | Build the layer surface with the parallel builder and with the serial oracle for: a full-document layer, an offset/partial layer, a Background without alpha, a layer with a missing channel, and a zero-size layer. | Both builders return identical `RgbaImage` values (width, height, and every pixel); the zero-size case is empty and does not panic. | `pictura-paint::layer_rgba_matches_serial_builder`, `pictura-paint::zero_size_sources_are_empty` |
| `ss_composite_parity` | Turn a `PixelBuffer` composite into a surface with the parallel builder and the serial oracle, including a zero-size composite. | Byte-identical `RgbaImage`; the zero-size case is empty and does not panic. | `pictura-paint::surface_from_composite_matches_serial_builder`, `pictura-paint::zero_size_sources_are_empty` |
| `ss_history_parity` | Capture a document, move its layer, then build the past state's document-space surface with both builders; include a zero-size document. | Past pixels land at the past rectangle and both surfaces are identical; the zero-size case is empty and does not panic. | `pictura-paint::layer_surface_matches_serial_builder`, `pictura-paint::zero_size_sources_are_empty` |
| `ss_profile` | Run `retouch_stroke_start_profile` on an 8000×6000 document for Brush control, Dodge, Smudge + Sample All, Clone Stamp All Layers, History Brush, and Pattern Stamp. | One timing line per tool. | `pictura-app::retouch_stroke_start_profile` |
| `ss_sample_all_parity` | Build the layer-local Sample All source with the one-pass builder and with the two-pass build-then-crop path for an offset/partial layer, including a zero-size layer rectangle. | Identical sampled pixels; the zero-size case is empty and does not panic. | `pictura-paint::sampled_surface_matches_two_pass_crop`, `pictura-paint::zero_size_sources_are_empty` |
| `ss_pattern_parity` | Paint the same Pattern Stamp stroke from a lazy pattern source and from a pre-tiled document-sized image, with the origin on the document origin, off it, and negative, including on a layer that hangs off the document; also request a zero-size tile. | Identical painted documents, including the off-canvas layer; the zero-size tile produces no source. | `pictura-paint::pattern_source_matches_tiled_image` |
| `ss_zero_size` | Build a layer surface, a composite surface, a history surface, and a crop with zero width or height. | Each builder returns an empty image and does not panic. | `pictura-paint::zero_size_sources_are_empty` |
| `ss_history_reuse` | Start a History Brush stroke from a history state, commit it (adding a new state), then start a second stroke from the same source state and layer path; paint from both source surfaces. | The second `brush_source_surface` returns the same `Arc` (`Arc::ptr_eq`, or a test-only build counter does not advance) and the painted documents are equal. | `pictura-app::brush_source_surface_is_reused` |
| `ss_history_invalidate` | Read the source surface, then in turn use a different source state/snapshot, make the source unresolvable or change its resolved id, change the brush source, change the layer path, and add or remove a snapshot; read again after each and compare the rebuilt image to `layer_surface` of the current source. Also confirm a source pinned when the stack drops it keeps its surface. | Each change invalidates the cache and the next read rebuilds from the current source; a plain commit (a new state) keeps the hit; a pinned source keeps the hit. | `pictura-app::brush_source_surface_invalidates_on_history_change`, `pictura-app::brush_source_surface_reuses_a_pinned_source`, `pictura-app::brush_source_surface_reuses_a_source_dropped_by_depth` |

## Manual checks

None. The profile is print-only and runs under `--ignored`; the numbers are
recorded in `design.md` rather than asserted.
