# Proposal: Linked layers transform as one and keep their link

## Why

Linking two layers makes the Move tool translate them together (#959), but Free
Transform still targets only the layer the session began on. Rotating a linked
layer turns one member, leaves the other behind, and then clears the link set,
so the two layers are no longer linked afterwards. Photoshop keeps a link set
rigid through every transform (scale, rotate, skew, distort, perspective, and
warp) and retains the relationship until the user unlinks it.

## What Changes

- A Free Transform session started on a linked layer includes every
  transformable member of its link set. The box is the union of the members'
  content rects and the reference point starts at its centre.
- Hidden linked members transform with the set; a position-locked or
  non-transformable member (group, adjustment, Background) is left in place,
  matching the Move tool.
- The moving preview composites the visible members over transparency; the
  commit applies the same map to each member (a per-member quad for projective
  modes, the same warp mesh for Warp).
- A transform commit no longer clears the link set. Links are path-keyed and a
  transform does not change layer paths, so they remain valid.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/free-transform`: the session can target a link set, and the link
  survives the commit.

## Impact

- Rust: `document_session/transform.rs`, `impl_transform/session.rs`,
  `impl_transform/session/preview.rs`, `impl_transform/warp/imp.rs`,
  `impl_core/deferred_composite/imp.rs`, `document_session/session.rs`.
- Tests: `document_session/tests.rs`, `tst_linked_layers.cpp`.
- No new dependencies.
