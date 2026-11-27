## Why

Edit > Puppet Warp has a disabled menu entry. A pin-based mesh lets users anchor part of a raster layer and reposition another part without changing tools.

## What Changes

- Add a modal Puppet Warp canvas with a live triangulated mesh, draggable pins, and explicit Apply and Cancel.
- Add stiffness, mesh density, edge expansion, mesh visibility, pin stacking, and rotation controls, with deterministic overlap ordering and bounded rendering work.
- Commit the resulting raster once and make it undoable. Bound the initial release to eligible 8-bit RGB and Grayscale raster layers and raster user masks.

## Capabilities

### New Capabilities

- `tools/puppet-warp`: interactive pin-driven mesh deformation and transactional raster application.

### Modified Capabilities

None.

## Impact

Edit commands, the Qt dialog, raster warp math, and the app's layer/history bridge. No new dependencies or serialized mesh data.
