# Tasks

## 1. Tests

- [x] 1.1 Rust: a shape layer moves, rotates, takes a quad and a warp frame as vectors; a position lock refuses.
- [x] 1.2 Rust: Free Transform's target check accepts a shape layer unless position-locked.
- [x] 1.3 Qt: the shape row has one thumbnail; the Move tool moves a shape; the transform commands map its outline.

## 2. Implementation

- [x] 2.1 Add `shape_layer_rect` and `map_shape_layer`, and branch translate, similarity, quad, and warp to them.
- [x] 2.2 Make the controls box (`content_rect`) hug a shape's outline.
- [x] 2.3 Accept a shape layer as a Move and transform target, with its outline as the session box.
- [x] 2.4 Drop the shape row's vector-mask thumbnail.
