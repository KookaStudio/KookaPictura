# Proposal

## Why

Issue #1156. On a 7.3 GB machine, creating a 17000×17000 document and then
resizing an open image to the same size was OOM-killed by the kernel. Each
17000² RGB document keeps about 3.8 GB resident (layer planes, `doc.composite`,
the sRGB level-0 frame, and the view pyramid), so two of them plus the rest of
the session exhaust RAM.

The app already refuses a document that does not fit `ImageBudget`
(`document_budget_allows`): New Document and the control server's
`image.image.size` / `image.canvas.size` both call it. The interactive Image
Size handler and the `PictureView::resize_image` bridge call
`pictura_render::resize_document` directly, so they allocate without the gate.
The default budget also never drops below one 2 GiB allocation, so on the
reporter's 456 MB it would still admit the 17000² target; the resize gate
therefore bounds the document by the memory available now.
`docs/01-architecture/performance-targets.md` requires a memory ceiling to raise
a typed error, never a silent OOM kill.

## What Changes

- Image ▸ Image Size SHALL refuse a target size that does not fit the
  `ImageBudget` caps or the memory available now, before resampling, leaving the
  document and its history unchanged.
- The `PictureView::resize_image` bridge SHALL apply the same gate.
- The Image Size handler SHALL show a warning naming the target size and the
  memory shortfall instead of silently doing nothing.
- The budget predicate moves to one shared module (`cxxqt_object/helpers.rs`),
  so New Document, Image Size, the resize bridge, and the control server use the
  same caps.

## Capabilities

### New Capabilities

- `document/image-resize-memory-budget`

### Modified Capabilities

None.

## Impact

`pictura-app`: `image_adjust/image_size`, `image_adjust/new_document`,
`impl_transform/dispatch`, the shared budget predicates in `helpers.rs`,
`cpp/image_size_dialog.*`, and `cpp/frame_menus_image.cpp`.

`docs/01-architecture/performance-targets.md` already specifies the ceiling; no
`docs/` change is needed. Out of scope: the interactive Canvas Size handler
(its control-server path is already gated) and scratch paging (#943).
