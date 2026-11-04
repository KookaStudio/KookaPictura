# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `mff_unregistered` | Composite a proxy-less type layer whose family is not registered | The region carries text coverage; it is not fully opaque | `pictura-render::missing_family_renders_with_the_bundled_face` |
| `mff_retry` | Call the two-step paint helper where the first face paints nothing | The bundled face is tried second and its result is returned | `pictura-render::a_face_that_paints_nothing_retries_with_the_bundled_face` |
| `mff_stored` | Composite a type layer in a missing family, then read its spec | The spec still names the missing family | `pictura-render::a_missing_family_keeps_its_stored_name` |
| Proxy-less type renders coverage | Composite a type layer with a style and no colour channel | Non-empty text coverage, not an opaque black rect | `pictura-render::proxy_less_type_layer_composites_live_coverage` |
| A stored proxy is unchanged | Composite a type layer that carries a colour channel | The channel's pixels are used | `pictura-render::type_layer_with_channel_composites_from_the_raster` |
| `tlc_no_ink` | Composite a blank-text proxy-less type layer over a solid base | The base is unchanged under the layer rect | `pictura-render::proxy_less_type_that_paints_nothing_leaves_the_backdrop` |
