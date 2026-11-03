# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `smudge_live_sample` | Run a Sample All Layers Smudge on a transparent target over a red/blue composite, from the red area into the blue | The far end carries the red the stroke laid, not the press-time blue | `pictura-paint::sample_all_layers_picks_up_the_live_layer` |
| `smudge_live_sample` (GUI) | On a single opaque layer, drag the same path with Sample All Layers on and off | Both strokes smear the same distance; the Sample All Layers stroke does not stop short | `tst_retouch_tools::smudgeSampleAllMatchesTheLiveLayer` |
| Dragging colour out of a bar | Drag from a black bar into white at Strength 90 % | One "Smudge" state; black carried into the white | `tst_retouch_tools::smudgeTool` |
| Finger Painting | Drag over white with Finger Painting and a red foreground | Red dragged into the white | `tst_retouch_tools::smudgeTool` |
