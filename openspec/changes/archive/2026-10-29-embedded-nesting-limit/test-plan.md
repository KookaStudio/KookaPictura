# Test Plan

One row per scenario in the delta specs. Every row is automated.

## compositing/smart-object-rendering

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| sor_nesting_limit | Composite a top-level no-proxy embedded object whose payload wraps `source_doc` in `MAX_EMBED_NESTING - 1` no-proxy, no-composite wrappers. | The centre pixel is the source's `[200, 50, 0, 255]`. | `pictura-render::nesting_up_to_the_limit_renders_the_innermost_source` |
| sor_nesting_past_limit | The same with `MAX_EMBED_NESTING` wrappers. | The centre pixel's alpha is 0. | `pictura-render::nesting_past_the_limit_stops_decoding` |
| sor_nesting_small_stack | Composite a 200-wrapper payload inside a 2-thread rayon pool with 512 KiB stacks, under a 60 s watchdog. | The render finishes, and the centre alpha is 0. | `pictura-render::a_deeply_nested_payload_renders_on_a_small_pool_stack` |

## compositing/layer-source-cache

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| lsc_self_reentry | On a plain thread under a 30 s watchdog, `lookup` a key whose build looks up the same key. | Both return `Artifact::Missing` without timing out. | `pictura-render::a_build_that_re_enters_its_own_key_does_not_wait_on_itself` |
