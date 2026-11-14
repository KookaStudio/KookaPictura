# Test plan

Every scenario is automated at the engine or Qt Test layer. The release-time
measurement is not a pass/fail contract; `scripts/profile-tools.py` records it
for the PR.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `mgc_magic_region` | On a textured 200×120 document, click one pixel with tolerance 0 and no layer style; compare the composite with a full CPU recomposite, check the frame revision, the history count and undo. | Frame unchanged, composite exact, one "Magic Eraser" state, undo restores. | `tst_tool_commit::aMagicEraserCompositesItsRegion` |
| `mgc_magic_styled` | Add a Color Overlay to a layer, then erase a pixel with the Magic Eraser. | The frame is rebuilt and the composite equals a full recomposite. | `tst_tool_commit::aStyledMagicEraserRecompositesWhole` |
| `mgc_magic_job` | Arm a 4.2 MP document, start a Magic Eraser click, cancel the document job, land it; check pixels, history index and Fade availability. | Nothing changed. | `tst_tool_commit::aCancelledMagicEraserJobChangesNothing` |
| `mgc_uniform_tiles` | Build a uniform layer with a tiled uniform alpha plane, erase through a whole-canvas mask at full opacity; inspect the alpha tiles and the dirty rectangle. | Every alpha tile is `Uniform(0)`, the dirty rectangle is the canvas, pixels equal the per-pixel reference. | `pictura-paint::a_uniform_erase_writes_uniform_alpha_tiles` |
| `mgc_uniform_parity` | Erase the same uniform layer at 50 % opacity and through an anti-aliased mask; compare every channel with the per-pixel reference. | Every channel equal. | `pictura-paint::a_uniform_erase_matches_the_per_pixel_path`, `pictura-paint::magic_erase_tiled_matches_dense` |
| `mgc_uniform_alpha` | Turn a Background into a layer; give a layer without transparency an alpha channel; inspect the tiles. | Every alpha tile is `Uniform(255)`, no dense tile, bytes opaque. | `pictura-render::layer_from_background_gives_a_uniform_alpha_plane`, `pictura-paint::ensure_alpha_gives_a_uniform_plane` |
| `mgc_opaque_fill` | Run `Pass::blend_one` with an opaque source at strength 1 in Normal mode for `Alpha::Replace`, `Keep` and `Blend`, and compare with the generic blend; draw a default Gradient and compare with the reference. | Every result equals the blend; the gradient matches. | `pictura-paint::an_opaque_full_strength_normal_fill_writes_the_source`, `pictura-paint::gradient::tests::a_linear_ramp_runs_from_start_to_end_and_extends_past_both` |

Manual rows: none. The 17000×17000 release measurement is recorded in the PR.
