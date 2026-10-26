# Spec Delta

## ADDED Requirements

### Requirement: Borrowed zero-copy level view

For a stored level below 0, the view SHALL expose that level's premultiplied
interleaved pixels as a std-only borrowed level handle that shares the level's
storage instead of copying the level. The handle keeps the storage alive; a
consumer MUST drop the handle when it is done. The Qt display image is built
app-side from this handle, so `pictura-render` gains no Qt dependency. When a
level is updated while a handle is alive, the handle MUST keep showing the bytes
it was created with and the update SHALL write to fresh storage (copy on write);
a handle created after the update SHALL show the new pixels. When no handle is
alive, an update MUST write in place and MUST NOT copy the level. Level 0 has no
stored interleaved buffer and MUST NOT be exposed as a borrowed handle.

#### Scenario: A borrowed view shares the level bytes [vp_view_shares_bytes]

- **WHEN** a borrowed level handle is requested for a stored level
- **THEN** its bytes equal that level's pixels and it reads the level's own
  storage rather than a new copy

#### Scenario: A view survives an update unchanged [vp_view_copies_on_write]

- **WHEN** a stored level is updated while a borrowed level handle of it is alive
- **THEN** the handle still shows the pixels it was created with, and a newly
  created handle shows the updated pixels

#### Scenario: An update without a view copies nothing [vp_view_update_in_place]

- **WHEN** a stored level is updated and no borrowed level handle of it is alive
- **THEN** the update writes the level's own storage in place and allocates no
  second level buffer

#### Scenario: Level 0 is not borrowed [vp_view_no_level0]

- **WHEN** a borrowed level handle is requested for level 0
- **THEN** the view reports that level 0 has no stored interleaved buffer rather
  than exposing the planar source
