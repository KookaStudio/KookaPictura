# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `abl_subject_colour` | Stack a red subject over a blue base with Seamless | The subject stays red | `pictura-render::seamless_stack_keeps_the_subject_colour` |
| `abl_focus_edge` | Stack a subject on a flat background | No halo beside the subject | `pictura-render::stack_focus_stops_at_subject_edges` |
| Horse and lily colour beside Photoshop | Blend the photographs at the same scale | Comparable hues | manual: no Photoshop oracle; tuned by eye |
