# Test Plan

Every scenario in the change's spec deltas maps to an automated Rust test. No
manual-only rows.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| A cloned sample store shares its allocation | Build a sample store at each depth, clone it, compare allocation identity | Clone and original share one sample allocation and equal bytes | `pictura-core::a_sample_store_clone_shares_its_allocation` |
| Writing a shared store forks only that store | Clone a sample store, write one clone through its mutable samples | Written clone has the new bytes, the other keeps the old bytes, allocations differ | `pictura-core::a_shared_sample_store_forks_on_write` |
| A unique store writes in place | Write a sample store with no other owner | Allocation is retained across the write | `pictura-core::a_unique_sample_store_writes_in_place` |
| Document clone shares retained samples | Clone a document holding a 16-bit `SourcePlanes` store | Both documents share the sample allocation | `pictura-core::a_document_clone_shares_retained_samples` |
| A bare PixelBuffer is 8-bit | Construct `PixelBuffer::new(2, 2, 3)` and read `data` | The data is a `Plane<u8>` of length 12 and the type is `PixelBuffer<u8>` | `pictura-core::a_bare_pixel_buffer_is_8_bit` |
| A wider sample buffer is expressible | Construct a `PixelBuffer<u16>` of width 2, height 2, and 3 channels | Its data is a `Plane<u16>` of length 12 | `pictura-core::a_wider_sample_buffer_is_expressible` |
| Cloning a document shares native samples and payload | Clone a document with a 16-bit layer store and an embedded smart-object payload | Clone equals the original and shares both allocations | `pictura-core::a_document_clone_shares_native_samples_and_payload` |
| A smart-object payload and its raw linked section share | Clone a document with an embedded smart object and a raw `layer_section_extra` record | The clone shares `SmartObject.payload` and `Document.layer_section_extra`, and both stay byte-identical | `pictura-core::a_smart_object_payload_and_raw_section_share` |
| A layer clone shares its byte blocks and layer tree | Clone a layer with `extra_blocks`, `raw_channels`, `blending_ranges`, mask, adjustment, smart-filter options, type descriptors, and children; write one side | Clone shares those block and child allocations; the write does not change the other side | `pictura-core::a_layer_clone_shares_blocks_and_children` |
| Preserved document byte blocks share on clone | Clone a document with `source_icc`, `document_icc`, `color_mode_data`, `image_resources`, `global_layer_mask`, and `layer_section_extra` | Every listed allocation is shared with the clone | `pictura-core::a_document_clone_shares_preserved_blocks` |
| A structural edit forks the tree | Clone a document, insert or reorder a layer in one copy | Edited copy has a new tree allocation; the other tree is unchanged | `pictura-core::a_clone_shares_and_a_structural_edit_forks` |
| A written store forks alone | Build a document with samples, a payload, a raw linked section, and a preserved block; write one store | Only the written store stops sharing; the others still share | `pictura-core::a_written_store_forks_alone` |
| Unchanged native samples are retained once | Capture many states of a 16-bit document without native edits | Every state shares one sample allocation per store | `pictura-app::native_samples_and_selection_are_shared_across_captures` |
| An unchanged selection is retained once | Capture many states without writing the selection | Every state shares one selection allocation | `pictura-app::native_samples_and_selection_are_shared_across_captures` |
| Unchanged payloads and byte blocks are retained once | Capture many states without editing a payload, raw linked section, or preserved block | Every state shares those allocations and their bytes do not grow with the capture count | `pictura-app::payloads_and_byte_blocks_are_shared_across_captures` |
| A capture forks the tree container but shares its nodes | Capture many states without a structural edit | Each capture may allocate a new tree container, but every node's bulk stores stay shared | `pictura-app::a_capture_forks_the_tree_container_but_shares_nodes` |
| A native edit retains only the changed plane | Capture, edit one plane of a per-channel native store, capture again | Only that plane's allocation is replaced; sibling planes stay shared | `pictura-app::a_native_plane_edit_forks_only_that_plane` |
| Undo of a native edit is byte-exact | Capture, edit a native store, the payload, the raw linked section, and the selection, undo | Restored document and selection equal the pre-edit bytes | `pictura-app::native_edit_undo_restores_old_bytes` |

## Regression coverage

The following existing suites must stay byte-identical and run in the same
change:

- `pictura-render::clipping_oracle`
- `pictura-render::adjustment_oracle`
- `pictura-render::native_depth_roundtrip`
- `pictura-app::capture_undo_round_trip_restores_doc_and_selection`
- `pictura-app::structural_capture_with_a_duplicated_layer_round_trips`
- `pictura-codec` depth and color-mode round-trip tests
