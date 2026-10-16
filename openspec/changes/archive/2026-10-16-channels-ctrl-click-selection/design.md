# Design

## Loading a colour row

The Channels panel's footer already loads an alpha channel as a selection
through `PictureView::load_selection("Alpha N")`. Ctrl-clicking a colour or
composite row needs the same "load a channel as a selection" behaviour, but a
colour channel is not a stored `doc.channels` entry — it is derived from the
composite. Rather than parse a row name in Rust (which would duplicate the
panel's mode→row model), the panel passes the row index it already resolved and
its visibility mask to a sibling bridge method:

```cpp
void ChannelsPanel::loadRowSelection(int row);   // alpha -> load_selection, else load_channel_selection
```

`PictureView::load_channel_selection(row, mask)` builds a document-sized
`Selection` from `current_buffer(doc, gpu_compute)` (sRGB-converted), using the
same per-row grayscale derivation as the panel thumbnails (`channelValue`):
row 0 is the composite luminance over the visible `mask` channels, and each
colour row is its own plane (RGB red/green/blue, CMYK / Multichannel naive ink,
everything else the composite luminance). The commit goes through
`apply_selection_labeled(.., CombineMode::New, "Load Selection")`, so it shares
the existing replace semantics, the reselect memory, and the history label with
`load_selection`.

## Why the panel passes the mask

The composite row must respect the channel eyes ("the visible channels"), which
are local view state held in `ChannelsPanel::mask_`. Passing it keeps the
visibility model in one place and lets the engine zero hidden channels exactly
as `ImageView` does with a multiply fill.

A colour row ignores the mask: CS6 loads a hidden channel's plane too, so the
mask only zeroes channels inside the composite row.
