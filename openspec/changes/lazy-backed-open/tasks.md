# Tasks

## 1. Tests named by the plan

- [ ] 1.1 `pictura-core`: `backed_tile_dropped_and_redecoded_identity`, `a_written_backed_tile_leaves_its_source`, `a_transient_read_leaves_a_backed_tile_away`, `backed_tiles_are_evicted_first_without_writes`.
- [ ] 1.2 `pictura-codec`: `lazy_open_matches_eager`, `lazy_open_decodes_no_layer_pixels`, `zip_and_deep_channels_open_eagerly`, `a_backed_tile_reads_only_its_band`, `lazy_index_rejects_short_channels`, `lazy_decode_of_a_truncated_source_is_unreadable`, `merged_preview_matches_the_merged_image`, `merged_preview_needs_alpha_or_background`, `source_replaced_keeps_backed_tiles`, `source_deleted_while_open`, `source_modified_in_place_marks_tiles_unreadable`.
- [ ] 1.3 `pictura-app`: `save_over_source_with_backed_tiles`, `first_paint_shows_the_files_merged_image`, `an_edit_replaces_the_merged_preview`, `picture_reads_layers_while_the_preview_shows`.

## 2. Core

- [ ] 2.1 `body.rs`: `Slot::Backed`, `Source`, `Body::backed`, detach on `dirty`, free drop in `pack`, transient pins.
- [ ] 2.2 `Plane::backed`, `TiledPlane::backed`.
- [ ] 2.3 `eviction.rs`: `Evictable::backed`, the backed class first; `memory::transient`.

## 3. Codec

- [ ] 3.1 `lazy.rs`: `SourcePin`, `ChannelIndex`, band decode and cache, `MergedPreview`.
- [ ] 3.2 `OpenOptions::source`; Phase B indexes eligible tasks; `read_psd_lazy` with the preview.

## 4. App

- [ ] 4.1 `decode_psd_with` pins the source and reads lazily; `OpenedDocument::preview` → `PictureViewRust::file_preview`.
- [ ] 4.2 `display_region` / `TileJob` read the preview; dropped on the first change; `from_tiles` refuses while set.
- [ ] 4.3 Layer thumbnails read under `transient`.

## 5. Measurement

- [ ] 5.1 M12 at 17000², eager and lazy, then under `MemoryMax=2G`; record in `docs/dev/STATE.md`.

## 6. Close-out

- [ ] 6.1 `docs/dev/STATE.md`, `ROADMAP.md`.
- [ ] 6.2 `openspec validate --all --strict`, `check-test-plan.py`, one build, `verify-fast`.
- [ ] 6.3 Archive the change.
