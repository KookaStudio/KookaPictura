# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A 16-bit layer color channel is narrowed | Read a depth-16 layered document | The layer channel is the narrowed samples | `pictura-codec::retained_store_is_typed_to_the_source_depth`, `pictura-codec::tagged_layer_info_reads_like_the_subsection` |
| A 16-bit mask channel is narrowed | Read the layered fixture with its mask stored each way | Identical layers | `pictura-codec::tagged_layer_info_reads_like_the_subsection` |
| A 16-bit unmodeled channel is narrowed for the engine and restored on save | Read and save the layered fixture | Channels survive at the source depth | `pictura-codec::layered_depth_fixtures_preserve_depth_and_channels` |
| Layers stored in a tagged block are narrowed the same way | Move a fixture's layer info into `Lr16`/`Lr32` and read it | Layers equal the original's, retained samples included | `pictura-codec::tagged_layer_info_reads_like_the_subsection` |
| Grouped layers are read from a tagged block | Read the grouped fixtures in tagged form | The group and its layers are present | `pictura-codec::tagged_layer_info_reads_like_the_subsection` |
| The parsed block leaves the preserved blocks | Place `Lr16` between two blocks | Only those two remain | `pictura-codec::tagged_layer_info_leaves_its_sibling_blocks` |
| A mismatched key stays preserved | Put `Lr32` in a depth-16 file | No layers; the block stays | `pictura-codec::tagged_layer_info_key_must_match_the_depth` |
| A PSB block with the 8B64 signature is read | Re-sign a saved PSB's block as `8B64` | Layers are read | `pictura-codec::psb_reads_tagged_layer_info_signed_8b64` |
| A malformed tagged block fails the read | Truncate the block's channel data; fuzz a tagged seed | An error, no panic | `pictura-codec::truncated_tagged_layer_info_is_an_error`, `pictura-codec::mutated_psds_never_panic_or_overallocate` |
| An unchanged high-depth document saves in tagged layout | Save the layered fixtures | Zero `info_len`, one block, equal on re-read | `pictura-codec::high_depth_layers_save_as_one_tagged_block` |
| A high-depth file with layers in the subsection still opens | Read the original fixtures | Layers are present | `pictura-codec::tagged_layer_info_reads_like_the_subsection` |
| An independent decoder reads tagged layers | Round-trip `rgb16_tagged.psd` through `psd-tools` | Same native samples | `pictura-codec::layered_depth_fixtures_preserve_depth_and_channels` |
| Photoshop opens a saved file | Open a 16-bit PSD in the app, save a copy, open the copy in Photoshop | All layers appear with the same pixels | manual: Photoshop is closed-source and not available to CI |
