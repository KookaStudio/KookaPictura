# Test plan — large-layer-duplicate-responsiveness

One row per scenario in the delta spec; every automated row runs in CI.

```bash
cargo nextest run -p pictura-core plane
cargo nextest run -p pictura-render neutral content_rect
cargo nextest run -p pictura_app duplicate
python3 scripts/profile-tools.py --image worldmap.png --scenarios duplicate,duplicate-copy,move-copy,delete-copy --metrics
```

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `ldr_diverged` | Fracture, write a band, clone, re-fracture at another width | Every view reads the logical bytes | `pictura-core::divergence_survives_clone_and_refracture` |
| `ldr_bounds_exact` | Compute the content rect of alpha patterns and compare with a per-pixel box | Equal for every pattern | `pictura-render::content_rect_matches_a_per_pixel_scan` |
| `ldr_bounds_no_materialize` | Fracture, clone and write alpha so a band forks, then compute the content rect | The materialization counter is unchanged | `pictura-render::content_rect_does_not_materialize_a_diverged_plane` |
| `ldr_neutral_exact` | Duplicate each accepted layer and composite | Byte-identical to the composite before | `pictura-render::neutral_duplicates_preserve_composite` |
| `ldr_excluded` | Break each condition in turn | Not neutral | `pictura-render::excluded_duplicates_are_not_neutral` |
| `ldr_edit_inserted` | Pass neutral and non-neutral duplicates through the edit helper | Inserted, and the structural fallback | `pictura_app::a_neutral_duplicate_is_an_inserted_edit` |
| `ldr_undo_no_warm` | Activate Move on a layer, add a layer, undo | The press after activation hits the cache; the press after undo misses it, so undo did not rebuild it | `tst_move_feedback::undoLeavesTheMoveWarmAlone` |
| `ldr_release_alpha` | Duplicate a Background | The copy's alpha plane is 255 over its rect | `pictura-render::a_background_copy_gets_an_opaque_alpha_plane` |
