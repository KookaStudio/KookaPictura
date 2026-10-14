# Proposal: port-photorust-picture-frame

## Why

Kooka has no Picture Frame. Issue #227 asks to port photorust's
`core/src/filters/frame.rs` (about 2.1k lines, 12 tests), which implements
CS6's `Filter ▸ Render ▸ Picture Frame`: vines, flowers, leaves, and ruled or
moulded frames driven by a single `FrameOptions`.

The corpus records Picture Frame as a **CC 2014.2** entry, not CS6, and the
`render-filters` capability currently forbids surfacing it. The owner's scope
for #227 supersedes that: Picture Frame ships as a deliberate post-CS6
addition, and the requirement is amended so the menu surfaces it while `Flame`
and `Tree` stay unsurfaced.

## What Changes

- `pictura-filters` gains `render::picture_frame` and `FrameOptions`, a
  port of photorust's model. A frame is drawn from three procedural
  ingredients — a vine, flowers (22 shapes), and leaves (23 shapes) — or as a
  ruled/moulded border, with Basic (margin, size, arrangement, flower/leaf
  sizes and colours) and Advanced (Number of Lines, Thickness, Angle, Fade,
  Invert) controls. Everything is parametric geometry: no Adobe artwork is
  traced, so the provenance rule is satisfied by construction.
- `Filter::PictureFrame { options: FrameOptions }`, the `picture-frame` kind
  (22 slots), and its defaults are added and dispatched. Out-of-range
  parameters are rejected with `FilterError::InvalidParams` before any
  mutation.
- Picture Frame previews against the whole layer
  (`filter_preview_needs_whole_layer`): its size and inset are fractions of
  the buffer.
- `Filter ▸ Render ▸ Picture Frame…` opens a dedicated `PictureFrameDialog`
  with a frame picker, a preview, and Basic/Advanced option pages; Last Filter
  Settings reopens it prefilled.

## Capabilities

### Modified Capabilities

- `imaging/render-filters`: Picture Frame is surfaced; `Flame` and `Tree`
  remain unsurfaced.
- `imaging/filter-application`: Picture Frame is a compositing kernel that may
  add opacity, so the alpha-preservation requirement's "every other filter
  leaves the transparency channel bit-identical" clause is amended to admit it.
  This touches the same requirement as the open `filter-alpha-family-contracts`
  change (#442); the two deltas must be reconciled when they archive.

### New Capabilities

- `imaging/filter-app-ui` gains *Picture Frame dialog*: the dedicated dialog,
  frame picker, and whole-layer preview.

## Impact

- `pictura-filters` `render`; `pictura-app` `filter_map.rs`, `filter_tools.rs`,
  `filter_commands.cpp`, `command_tree.cpp`, `frame_menus_filter.cpp`, and the
  new `picture_frame_dialog.{h,cpp}` plus `tst_picture_frame_dialog`.
- **Output changes:** a new filter; no existing result changes and no golden
  baseline covers Picture Frame.
- **Oracle:** no equivalent. It is covered by property tests ported from
  photorust and by Qt Test cases for the dialog. No new self-test checks
  (rule 11).
- **Provenance:** all frame artwork is procedural parametric geometry ported
  from photorust (GPL-3.0); no traced Adobe assets.
- **Docs:** `docs/06-filters/render-filters.md` is amended to record Picture
  Frame as a shipped post-CS6 addition, in a separate `TASK-ALLOWS-DOCS`
  commit.
