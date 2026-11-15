# Design

## Coverage clip

`pictura_paint::CoverageClip` carries the active selection's coverage plane and
its width:

```rust
pub struct CoverageClip {
    pub data: Plane<u8>,
    pub width: u32,
}
```

The stencil already receives each changed pixel's layer-local index `i` and the
stroke's current layer rect, which is in document coordinates for a layer
stroke, a canvas-sized mask stroke, and Quick Mask alike. It maps the index to
document coordinates:

```
doc_x = rect.left + i % rect.width
doc_y = rect.top  + i / rect.width
clip  = data.sample(doc_y * width + doc_x)
```

The layer rect is the stroke's live rect, so a layer that grows under the brush
needs no clip remap. The clip is `None` without a selection; an all-255
selection is a no-op by multiplication.

- Preview: level pixel `(lx, ly)` covers document pixel `(lx * scale, ly *
  scale)`; the blend samples the clip there. The preview is approximate and is
  replaced by the exact commit.

## Per-dab engines

Color Replacement, Mixer Brush, Art History, Background Eraser, Blur/Sharpen,
Smudge, and Dodge/Burn/Sponge write their own working image instead of going
through the stencil. After each sample their changed rectangles are blended
toward the pre-sample layer pixels by the clip: zero coverage restores the
pixel, full coverage keeps the engine's result, and partial coverage lerps. The
working image is left masked, so the next sample starts from the clipped state.

## Backends

The GPU stroke patches the working layer from the shader and has no coverage
clip. While a selection is active the GPU dab is not taken; the exact CPU
stroke (or its reduced-level preview) runs instead, so committed pixels are
clipped. The preview blend multiplies the accumulated coverage by the clip
sample, so the live picture is clipped too.

## App wiring

`begin_kind` is the shared entry for the Brush, Pencil, Eraser, stamps, and
retouch strokes; it attaches the clip before storing the session. `begin_paint`
(the direct bridge entry) and `begin_mask_paint` do the same, and the Quick Mask
path switches to the new struct.
