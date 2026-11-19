# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pml_link_clears_bit` | Build a mask with flags `0xA1` and link it. | Flags become `0xA0`; the mask reads linked; bounds and coverage are unchanged. | `pictura-render::set_linked_round_trips_the_flag_bit_and_preserves_the_rest` |
| `pml_unlink_sets_bit` | Build a mask with flags `0xA0` and unlink it. | Flags become `0xA1`; the mask reads unlinked; bounds and coverage are unchanged. | `pictura-render::set_linked_round_trips_the_flag_bit_and_preserves_the_rest` |
| `pml_new_mask_linked` | Create each layer-mask kind through the mask command. | Every created mask reads linked with bit 0 clear. | `pictura-render::masks_created_by_commands_are_linked` |
| `pml_default_fill_mask_linked` | Call `default_layer_mask` with no selection and with a selection. | Both masks have bit 0 clear and read linked. | `pictura-app::default_masks_are_linked` |
| `pml_link_noop` | Set a mask to the linked or unlinked state it already has. | No change is reported and the document is unchanged. | `pictura-render::set_linked_round_trips_the_flag_bit_and_preserves_the_rest` |
