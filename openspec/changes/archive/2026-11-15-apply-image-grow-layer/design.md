# Design: apply-image-grow-layer

## Growth belongs to the app path

`apply_image_region` takes the layer and the write region; it does not know the
document. Growing inside the kernel would also grow after
`PreviewBaseline::save`, and a `Section` baseline refuses to restore when the
layer rect has changed. The filter path already grows the layer in
`apply_op_active_region` before saving its baseline and remembers the
pre-growth rect in `FilterPreview::rect`. Apply Image uses the same path, so it
grows there too.

The growth condition mirrors the kernel's `keep_alpha`:

```text
!params.preserve_transparency && !layer_transparency_locked(layer)
```

`grow_layer_to` refuses a transparency lock and a pixel lock; the pixel lock is
already refused by `pixel_filter_target`.

## Growth target

The destination grows to `layer.rect ∪ document rect`. The union keeps a layer
that already extends past the canvas a superset, which `grow_layer_to`
requires. An empty layer (0×0) grows to the document. A layer that is already
canvas-sized does not change.

The new area is transparent at every depth. `grow_layer_to` pads the 8-bit
channels, the retained native store, and drops verbatim raw channels (its
existing ceiling).

The active selection still confines the blend through `selected` coverage; it
does not confine the growth. This is an inference from the issue's wording
("extend a smaller layer across the document"), not an Adobe-verified detail.

## Preview

`preview_rect` is taken before the growth, as it is for filters. The baseline
is then saved against the grown layer:

- a native store takes the whole-layer baseline, because a section baseline
  holds 8-bit channels only;
- an 8-bit layer takes the section baseline, whose `layer_rect` is the grown
  rect.

On the next preview, `baseline.restore` runs first and then
`shrink_layer_to(preview.rect)` puts the original rect back. Cancel does the
same. A commit re-grows from the restored original and keeps the grown rect.
The refresh region is the section for a preview and the grown rect for a
commit; the grown padding is transparent before and after, so a section preview
does not need to repaint it.

## Kernel contract

`apply_image_region` keeps its current behaviour: the clip is
`region ∩ layer.rect`. Its `ponytail:` note about Photoshop growing the layer is
replaced by a note that the caller grows first. The integration test proves the
kernel writes the full grown area when the caller passes the document rect.

## Ceilings

- Growth is to the document, not to the selection's bounds.
- A transparency-locked layer keeps its rect; so does Preserve Transparency.
- The grown padding is not trimmed after a commit, matching the filter ceiling;
  a later Move or Free Transform trim drops it.
- The layer mask is not grown; its existing rect and default colour continue to
  define coverage outside the mask rect.
