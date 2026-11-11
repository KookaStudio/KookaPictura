## REMOVED Requirements

### Requirement: Borrowed zero-copy level view

**Reason**: Levels are no longer stored whole; the canvas presents tiles from the tile pyramid cache (`compositing/tile-pyramid`).
**Migration**: The canvas level provider's whole-level image is removed; `ImageView` presents crops assembled from tiles.
