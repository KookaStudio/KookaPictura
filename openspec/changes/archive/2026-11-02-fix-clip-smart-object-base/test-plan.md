# Test Plan

One row per scenario in the change's spec deltas. Coverage names the automated
test as `<crate-dir>::<fn>`; no row is manual.

| Scenario | Steps | Expected result | Coverage |
| --- | --- | --- | --- |
| `sor_merged_alpha` | Author a 2×2 RGB embedded payload whose merged transparency plane is opaque on one half and transparent on the other; call `render_smart_source` over the full rect. | The returned buffer is RGBA and its alpha equals the payload's transparency plane; transparent half is 0, opaque half is 255. | `pictura-render::source_preserves_merged_alpha` |
| `sor_merged_alpha_gray` | Author a 2×2 grayscale payload with a transparency plane; call `render_smart_source` over the full rect. | The returned buffer is RGBA with the gray value in RGB and the payload's plane as alpha. | `pictura-render::source_preserves_grayscale_merged_alpha` |
| `sor_merged_alpha_first_extra` | Author a 2×2 RGB embedded payload with two extra merged planes: a transparency plane first and a different plane second; call `render_smart_source` over the full rect. | The returned buffer's alpha equals the first extra plane, not the second. | `pictura-render::source_uses_first_extra_plane_as_alpha` |
| `sor_merged_alpha_absent` | Author an opaque 2×2 RGB payload with no extra merged plane; call `render_smart_source` over the full rect. | The returned buffer's alpha is 255 everywhere. | `pictura-render::source_without_extra_plane_is_opaque` |
| `clip_smart_source_base` | Composite a full-canvas green layer clipped to a proxy-less smart-object base whose embedded source is opaque on the left half and transparent on the right half. | Green appears on the left half only; the right half keeps the backdrop. | `pictura-render::a_clipped_layer_respects_a_smart_object_bases_alpha` |
| `clip_smart_proxy_base` | Composite a full-canvas green layer clipped to a smart-object base carrying proxy channels whose alpha is opaque on the left half and transparent on the right half. | Green appears on the left half only; the right half keeps the backdrop. | `pictura-render::a_clipped_layer_respects_a_proxy_bases_alpha` |

## Notes

- Regression teeth: `source_preserves_merged_alpha`,
  `source_preserves_grayscale_merged_alpha`,
  `source_uses_first_extra_plane_as_alpha`, and
  `a_clipped_layer_respects_a_smart_object_bases_alpha` fail without the fix.
- Guards: `source_without_extra_plane_is_opaque` and
  `a_clipped_layer_respects_a_proxy_bases_alpha` pin unchanged behavior.
- `clipping_smart_source_matches_cpu` is GPU parity only; CPU and GPU shared the
  same alpha before the fix, so it is not a regression test.
- Depth-1 Bitmap extras stay packed and are skipped; no synthetic Bitmap payload
  test (the pre-fix opaque behavior is preserved).
