## REMOVED Requirements

### Requirement: Halved-level view structure

**Reason**: Whole stored levels conflict with the production tile cache.

**Migration**: Use the logical level topology in compositing/tile-pyramid and pixel semantics in compositing/region-composite.

### Requirement: Damage-driven level update

**Reason**: Production repairs or invalidates cached tiles; no whole stored levels exist to update.

**Migration**: Use Damage invalidates and repairs tiles in compositing/tile-pyramid.

### Requirement: Viewport crop

**Reason**: The production crop is assembled from cached tiles rather than a stored-level structure.

**Migration**: Use The viewport presents tiles from a tile pyramid cache and the region-composite pixel contract.

### Requirement: Tile-aligned damage update

**Reason**: The retired oracle damage API is not a production repair contract.

**Migration**: Use tile-cache damage repair; the independent oracle supports only whole rebuilds and crops.

### Requirement: A banded level-0 source is read row by row

**Reason**: A production stored-level source API is no longer required.

**Migration**: Production reads regions or bands; retain row-based source reading in the test-only reference as covered by verification/view-pyramid-oracle.
