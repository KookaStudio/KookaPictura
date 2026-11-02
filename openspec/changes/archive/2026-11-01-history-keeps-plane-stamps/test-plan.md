# Test plan — history-keeps-plane-stamps

```bash
cargo nextest run -p pictura_app history
```

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `hpi_keeps` | Capture, write one channel, capture | Other stamps unchanged; written plane has a new nonzero stamp | `pictura_app::a_capture_keeps_the_stamps_of_unchanged_planes` |
| `hpi_undo` | Capture, write one channel, capture, undo | Unwritten planes keep their stamps | `pictura_app::undo_keeps_the_stamps_of_unchanged_planes` |
| `hpi_unique` | Clone a layer into the stack, capture twice | All nonzero stamps unique; the second capture keeps them | `pictura_app::a_duplicated_layer_gets_unique_stable_stamps` |
