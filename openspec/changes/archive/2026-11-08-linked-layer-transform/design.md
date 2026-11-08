# Design: Linked-layer transform

## Data

`TransformSession` keeps its primary `path` and `orig_rect`, and adds
`members: Vec<(String, PsdRect)>` in panel order. `orig_rect` is the union of
the member rects, so every existing gesture (pivot, scale, rotate, skew, quad
drag) works unchanged on the union box. `untrimmed` becomes a per-member list
so a cancel restores every layer's pre-trim extent.

## Begin

`prepare_transform` asks `transform_group` for the targets: `link_members`
(panel order) filtered to the primary plus members that pass
`can_transform_target(..., require_visible = false)`. Hidden members pass; a
position-locked or untransformable member does not. Each member is trimmed to
its content and its source rect is recorded. A lifted selection keeps the
single floating-layer path.

## Preview

The base hides every member (the existing `backdrop_patch` already accepts a
path slice). The moving image is the members composited over transparency,
cropped to `orig_rect`; hidden members are skipped in the image but still
included in the union. Opacity is baked into the group render (255). Warp
re-renders by applying the same warp mesh to each member in a document clone,
then compositing the group.

## Commit

The commit captures every member with its source rect. Similarity and skew use
`apply_session_transform` per member, which re-centres the engine's map on that
member's own rect while preserving the session's global pivot. Projective modes
solve the session homography once for the union rect and carry each member's
corners through it. Warp applies the same document-space mesh and frame to every
member. A group commit runs on a clone and installs only when every member
succeeds, so a refused member cannot leave a half-transformed set.

The deferred `TransformSpec` carries `paths` instead of one `path`; the worker
clones every transformed layer and `land_transform` installs them all. The edit
is a bounded region (or `Full` when a member has effects), which does not clear
link sets.

## Link persistence

The transform commit no longer calls `clear_link_sets`, and its edit is
`Regions`/`Full` rather than `Structural`, so the path-keyed link set survives.
A lifted selection still records a structural edit because it replaces the
document's floating layer.
