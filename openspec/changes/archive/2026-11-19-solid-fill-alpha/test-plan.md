# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A non-opaque fill encodes as the 4-byte tuple | Encode `FillContent::Solid([10, 20, 30, 40])` and `Solid([10, 20, 30, 0])` with the shared fill encoder and decode each. | `SoCo` payloads are `[10, 20, 30, 40]` and `[10, 20, 30, 0]`; `decode_fill` returns the same two fills. | `pictura-render::solid_fill_encoder_preserves_alpha` |
| An opaque fill still encodes as the descriptor | Encode `FillContent::Solid([10, 20, 30, 255])` and decode it. | Payload is a version-16 descriptor; `decode_fill` returns `Solid([10, 20, 30, 255])`. | `pictura-render::solid_fill_encoder_preserves_alpha` |
| A new fill layer carries the standard descriptor | `add_solid_fill(doc, "", [10, 20, 30, 255])`. | Layer decodes to `Solid([10, 20, 30, 255])`, its `SoCo` payload is the descriptor, and the composite is opaque. | `pictura-render::created_solid_fill_layer_preserves_alpha` |
| A new translucent fill layer keeps its alpha | `add_solid_fill(doc, "", [10, 20, 30, 40])` and `[10, 20, 30, 0]`, then composite each. | Alpha 40 decodes and composites to `[10, 20, 30, 40]`; alpha 0 composites transparent. | `pictura-render::created_solid_fill_layer_preserves_alpha` |
| A translucent fill survives save and reopen | `write_psd` then `read_psd` on the translucent-fill document. | The reopened layer decodes to `Solid([10, 20, 30, 40])`. | `pictura-render::created_solid_fill_layer_preserves_alpha` |
| An in-house 4-byte fill still decodes | Decode a hand-built 4-byte `SoCo` block. | `decode_fill` returns `Solid([10, 20, 30, 40])`. | `pictura-codec::fill_decoders_preserve_parameter_contract` |
| `scf_panel_foreground` | Trigger `Solid Color…` from the Layers panel strip with a non-black foreground. | The new layer composites to that foreground colour. | `tst_layers_panel_fixes::solidFillUsesForeground` |
| `scf_menu_foreground` | Dispatch the Layer > New Fill Layer > Solid Color command with a non-black foreground. | The new layer composites to that foreground colour. | `tst_layer_commands::solidFillCommandUsesForeground` |
| `scf_default_black` | Trigger `Solid Color…` with the default black/white pair. | The new layer is opaque black, not white. | `tst_layers_panel_fixes::solidFillUsesForegroundNotBackground` |
