# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `abl_blend_limit` | Blend layers spanning more than 100 MP | Masks added, no merged layer | `pictura-render::merged_layer_is_skipped_above_the_blend_limit` |
| Masks-only notice | Run Auto-Blend on layers spanning more than 100 MP | The status bar explains that only masks were added | manual: a real 100 MP run is too heavy for the offscreen suite |
