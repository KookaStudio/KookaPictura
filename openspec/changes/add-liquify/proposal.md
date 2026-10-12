# Proposal: add-liquify

## Why

Issue #348 (part of the #167 remaining-filter audit). `Filter ▸ Liquify…`
(Shift+Ctrl+X) was a disabled stub with no kernel and no photorust source.
The docs contract (`docs/06-filters/liquify.md`, `FILT-090`) describes a
mesh-warp workspace: Forward Warp, Reconstruct, Twirl, Pucker, Bloat, Push
Left, Mirror, Turbulence, Freeze and Thaw masks, Basic and Advanced modes,
Reconstruct / Restore All, and CS6's Load Last Mesh. photocraft (MIT OR
Apache-2.0) has a displacement-field Liquify whose design this ports.

## What Changes

- `pictura-filters` adds `distort::liquify`: `LiquifyMesh` (an inverse
  displacement per node of a 2/4/8 px grid plus a freeze mask), `LiquifyBrush`
  and `LiquifyTool`, the dab model ported from photocraft (each tool reads the
  current output at `q = M(p)`, so `d' = q + d(q) - p`), Kooka's own Mirror and
  Turbulence, Reconstruct / Restore All / Mask All / None / Invert All, a
  scale-to-fit, the commit resampler `liquify` (Catmull-Rom mesh and source,
  untouched rows copied), and `LiquifySource`, which renders the dialog's view
  at screen resolution from the full-resolution layer.
  `Filter::Liquify { mesh }` dispatches to it.
- `pictura-render` lists Liquify in `filter_preserves_opacity` (the alpha
  plane is warped with the colour).
- `pictura-app` adds a `filter_tools/liquify.rs` bridge: one dialog session
  over the active layer (its view rendered at screen resolution, re-rendered
  per dab rectangle) and a store of kept meshes. The `liquify` kind is one slot, the
  kept mesh's id, so OK commits through the ordinary filter path (one history
  state, Last Filter, Last Filter Settings). The C++ `LiquifyDialog` /
  `LiquifyCanvas` give CS6's workspace.
- Ceilings: Adobe's mesh math is closed, so the tool strengths are tuned by
  eye; no `.msh` Save/Load Mesh, no mask from a selection or channel, no
  Show Guides, Blend backdrop mode is a Difference stand-in, the Reconstruct amount is applied on OK rather
  than previewed, kept meshes live for the process only, and there is no GPU
  path. `Edit ▸ Fade` is an app-wide stub; Liquify commits as a filter so it
  is covered when Fade lands.

## Capabilities

### New Capabilities

- `imaging/liquify-filter`: the mesh, the tools and their directions, the
  freeze mask, reconstruction, scale-to-fit, and the resampler's identity
  and translation guarantees.
- `imaging/filter-app-ui` gains *Liquify workspace*: the menu row, Basic and
  Advanced modes, shortcuts, the session preview, OK / Cancel / Load Last
  Mesh, and the one-slot kind.

## Impact

- `pictura-filters` (`distort/liquify.rs`, `liquify/resample.rs`,
  `liquify/tests.rs`, `Filter`, `apply`, exports), `pictura-render`
  (`filter_preserves_opacity`), `pictura-app` (`filter_tools/liquify.rs`,
  `filter_map.rs`, `arities.rs`, `build.rs`, `filter_commands.cpp`,
  `frame_menus_filter.cpp`, `liquify_dialog.*`, `liquify_canvas.*`,
  `CMakeLists.txt`), `tst_liquify_dialog`.
