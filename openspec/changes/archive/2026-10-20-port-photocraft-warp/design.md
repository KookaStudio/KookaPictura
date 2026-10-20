# Design — Port Edit ▸ Transform ▸ Warp from photocraft

## Source

photocraft (<https://github.com/storytold/photocraft>, MIT OR Apache-2.0, taken under MIT;
see `NOTICE.md`):

| photocraft | Kooka |
|---|---|
| `crates/geom/src/warp.rs` | `pictura-core/src/warp.rs`, `warp/mesh.rs`, `warp/tests.rs` |
| `crates/algo/src/warp.rs` (`warp_mesh_surface`, `warp_triangles`, `warp_mesh_gray`) | `layer_ops/warp_raster.rs` |
| `crates/engine/src/warp_cmds.rs` (`warp_layer`) | `layer_ops/warp.rs` |
| `crates/ui-egui/src/transform_tool.rs` (Warp mode) | `impl_transform/warp.rs`, `tool_transform.cpp`, `options_bar_transform.cpp`, `image_view_overlays.cpp` |

## Decisions

- **CS6 scope.** Split Warp (crosswise, vertical, horizontal) and the Grid
  menu are Photoshop CC 2020 features, so they are not ported.
  `remove_split_*` is dropped. `split_u` / `split_v` stay because
  `Warp::to_mesh` uses them to subdivide a preset exactly.
- **Planar, premultiplied.** photocraft rasterizes interleaved tiles. Kooka
  keeps channels as planes, so the resampler converts each plane to unit `f32`,
  premultiplies by the `-1` plane, and samples every plane with one set of tap
  weights. Taps outside the source read zero, as photocraft's do, so warped
  edges are antialiased. Single samples use Kooka's interpolation kernels
  (Catmull–Rom bicubic, which matches photocraft's Keys bicubic, plus Smoother
  and Sharper). The shrink path keeps photocraft's bilinear area subsamples.
- **Row bands.** Work splits into 64-row destination bands, with triangles
  binned by y and processed in parallel with rayon. photocraft splits into
  tiles instead.
- **Masks.** The layer mask goes through its own triangulation, with full
  coverage, and is flattened onto its default colour (`warp_mesh_gray`). A
  16/32-bit retained store goes through the same triangles as the 8-bit
  channels, so both stay in register.
- **Session.** As in photocraft, Warp replaces the box. Entering Warp maps the
  identity one-patch mesh through the box's homography. Choosing a preset
  starts it over the session's source box, and the commit applies only the
  warp. Leaving Warp drops the warp edits and restores the box.
- **Preview.** Each edit warps the layer on a document clone and rebuilds only
  the moving image; the base stays cached. This is a `ponytail:` ceiling,
  because a large layer drags slowly. photocraft draws a GPU texture mesh
  instead.
- **Parity.** The preset shapes are photocraft's fits to Photoshop renders. They
  are approximations, not verified parity: the Photoshop-sample test checks
  them against measured points within a tolerance.
