# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| pattern_define | Define a selected asymmetric tile, restart list, use it. | Pixels and identity survive; no history entry. | `tst_pattern_authoring::defineAndReuse` |
| pattern_define_cancel | Try absent, feathered and nonrectangular selection and cancel naming. | No preset or document change. | `tst_pattern_authoring::invalidAndCancel` |
| pattern_consumers | Define a tile and use it in Bucket, Stamp, Pattern Fill and shape fill. | Each uses the tile; layer and shape patterns embed and resolve. | `tst_pattern_authoring::consumerReuse`, `tst_pattern_authoring::shapePatternAtomicity` |
| pattern_define_failure | Inject persistence and memory failures; separately change the active document, selection or pixels during naming. | No preset, pixels or history change. | `tst_pattern_authoring::defineFailure`, `tst_pattern_authoring::captureMemoryAndStaleEdit` |
| pattern_fill_preset | Fill a selection with defined and imported tiles, then undo and alter list. | Correct repeat, clipping and undo; result stable. | `tst_pattern_authoring::fillPreset`, `pictura-paint::pattern_clear_respects_source_alpha`, `pictura-paint::scripted_clear_leaves_gaps_untouched`, `pictura-paint::local_pattern_surface_uses_document_origin` |
| pattern_fill_empty | Empty current patterns and open Fill. | Pattern operation disabled; no edit. | `tst_pattern_authoring::emptyList` |
| pattern_fill_replay | Record ordinary and scripted fills, rename or remove the preset, then replay; replay a legacy index-only action. | Recorded snapshots and script choices reproduce; legacy built-in index still resolves. | `tst_pattern_authoring::fillReplay`, `tst_pattern_authoring::builtinSnapshot` |
| pattern_scripts | Render each script twice with an asymmetric tile in a sufficiently large region; inspect each layout. | Deterministic; brick stagger, perpendicular weave, scatter, outward spiral and mirrored symmetry are observable. | `pictura-app::scripted_pattern_geometry_and_determinism` |
| pattern_script_bounds | Fill a small region and request an operation above admitted work or allocation bounds. | Small result is deterministic; excessive operation is refused without partial edit or history. | `pictura-app::scripted_pattern_bounds`, `tst_pattern_authoring::scriptBounds` |
| pattern_script_edit | Fill with selection, opacity and transparency lock, then undo. | Clipping and alpha policy honored; undo restores. | `tst_pattern_authoring::scriptEdit` |
| pattern_script_cancel | Cancel dialog after selecting a script. | No pixels or history change. | `tst_pattern_authoring::scriptCancel` |
