# Test Plan

One row per scenario in `specs/compositing/cpu-kernel-parallelism/spec.md`.
`FORCE_SERIAL` is a `#[cfg(test)]` switch in each crate, so the
serial-versus-parallel tests are unit tests: an integration test links the
library built without `cfg(test)`, where the switch does not exist.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pk_parallel` | Apply every applicable `Adjustment` kind (the three fill variants return `Unsupported` and are skipped) to the same random buffer with the serial switch on and off | The two buffers are byte-identical | `pictura-adjust::parallel_matches_serial` |
| `pk_region` | Composite a document with an adjustment layer over a region of 128 rows or fewer, once with the render crate's `FORCE_SERIAL` and once without | The two buffers are byte-identical; the adjustment kernel itself is covered by `pk_parallel` | `pictura-render::short_region_composite_matches_serial` |
| `pk_display` | Convert a 3-channel and a 4-channel Adobe RGB buffer for display with the parallel path and with the serial switch | The two buffers are byte-identical, and a 4-channel buffer keeps its alpha plane | `pictura-codec::parallel_display_transform_matches_serial`, `pictura-color::byte_transform_cache_flag_does_not_change_output` |
| `pk_serial` | Enable each crate's serial switch and run the same input through the parallel and serial branch of every touched kernel | Every output byte is identical | `pictura-adjust::parallel_matches_serial`, `pictura-render::parallel_composite_equals_sequential`, `pictura-render::clipping_parallel_matches_serial`, `pictura-codec::parallel_display_transform_matches_serial` |
| `pk_matte` | Blur a random matte as one plane and as three identical planes over several sizes and blur radii | The two results are byte-identical | `pictura-render::one_plane_matte_blur_matches_three_planes` |
| `pk_effects` | Render drop shadow, inner shadow, outer glow, inner glow, bevel, satin, overlay and stroke with the serial switch on and off | The rendered effects are byte-identical | `pictura-render::every_effect_parallel_matches_serial` |
| `pk_unchanged` | Run the existing known-value, differential and structural oracles after the change | Every output buffer and encoded file matches its recorded result | `pictura-adjust::levels_matches_imagemagick`, `pictura-render::size_softens_the_shadow`, `pictura-render::psd_tools_composites_a_clipping_group_alike`, `pictura-codec::embedded_icc_profile_is_applied_and_dropped` |

## Manual checks

None. Every scenario is automated; the change is execution-only and has no
visual or interactive surface.
