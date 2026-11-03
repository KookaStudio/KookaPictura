# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `abl_flat_base` | Stack a flat background over a bright base | Flat areas masked out, detail kept | `pictura-render::stack_images_drops_a_flat_upper_background` |
| `abl_upper_detail` | Stack a subject over busy texture | The subject is kept | `pictura-render::stack_images_keeps_upper_detail_over_busy_texture` |
| `abl_base_tone` | Stack with Seamless on | Tone from the base, detail kept | `pictura-render::seamless_stack_takes_tone_from_the_base` |
| Horse and lily beside Photoshop | Blend the two photographs with Stack Images and Seamless | Comparable tone and layout | manual: no Photoshop oracle; tuned by eye |
