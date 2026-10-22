# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `wt_bounded_decode` | Decode warp descriptors with bend 1e308 and NaN | 100 and 0 | `pictura-codec::decodes_the_warp_descriptor` |
| `wt_bounded_render` | Place Arc type with infinite, NaN, and 1e308 values | Refused or within 4× the unwarped span; out-of-range equals ±100 % | `pictura-render::hostile_warp_values_stay_bounded` |
