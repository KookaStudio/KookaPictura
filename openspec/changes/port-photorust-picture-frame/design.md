# Design: port-photorust-picture-frame

## Model

Ported from perfecto25/photorust `core/src/filters/frame.rs`, with its
constants and idioms kept so the port diffs against upstream.

- **Rasteriser.** Each motif is a set of filled contours scan-converted with
  four sub-scanlines per row (vertical anti-aliasing) and fractional end
  coverage (horizontal). Contours are wound so the non-zero rule cuts holes
  (`solid` / `hole`). The whole frame is drawn onto a clear RGBA sheet first,
  then composited over the picture once so Fade thins the frame as a whole.
- **Geometry.** Circles, ellipses, rectangles, polar curves, stars, petals,
  arc bands, dashed rings, round-joined strokes, quadratic Béziers, and
  tendril curls. A motif sits on a rounded-rectangle `Track`; `at(s)` walks it
  clockwise from the top-left and `spread` spaces positions evenly.
- **Catalogue.** 47 frames (`FRAME_NAMES`), 22 flowers (`FLOWER_NAMES`), 23
  leaves (`LEAF_NAMES`). Frames 1–35 are motif recipes; 36–47 are ruled
  (36–41) and moulded (42–47) borders drawn from a corner distance field so
  the lines stay parallel at any corner shape.
- **Seeded arrangement.** A small xorshift `Rng` seeded from
  `arrangement` and `frame`, so the same arrangement reproduces the same
  frame. `Angle` turns every flower and leaf; `Invert` flips the inward
  normal (motif frames) or the moulding profile.
- **Scale.** Everything is a fraction of the canvas's shorter side
  (`MARGIN_BASE`, `MARGIN_PER_STEP`, `UNIT_AT_FULL`, `FLOWER_AT_FULL`,
  `LEAF_AT_FULL`, `VINE_SHARE`, `BAND_AT_FULL`, `CORNER_PER_BAND`), so a frame
  on a proxy and on the full layer is the same picture.

## Kooka adaptations

- The engine runs on the interleaved `photorust::pixmap::Pixmap` (the same
  image the other photorust ports use) and is dispatched from
  `photorust::dispatch`, which converts to and from the planar buffer.
  `FrameOptions` carries colours as `[u8; 3]`, converted to `Rgba8` at draw
  time. `source_over` moves from photorust's `brush` module into
  `photorust/pixmap.rs`.
- `Filter::PictureFrame` joins `filters_alpha`: a frame is composited with
  source-over, so it can add opacity where the layer was transparent. That is a
  compositing write, not a spatial move, so the `filter-application`
  alpha-preservation requirement is amended to name it alongside the moving
  kernels; the delta carries #442's amendments to the same requirement so the
  two archive in either order.
- Parameter ranges (`FRAMES`, `MARGIN`, `SIZE`, `ARRANGEMENT`, `FLOWER`,
  `LEAF`, `FLOWER_SIZE`, `LEAF_SIZE`, `LINES`, `THICKNESS`, `ANGLE`, `FADE`)
  are validated in `dispatch::plan` and rejected as
  `FilterError::InvalidParams`, rather than photorust's silent clamp. The
  engine still clamps internally as a defensive ceiling.

## Slots

The `picture-frame` kind is 22 slots, in the dialog's control order:

`frame, vine r, g, b, margin, size, arrangement, flower, flower r, g, b,
flower size, leaf, leaf r, g, b, leaf size, lines, thickness, angle, fade,
invert`

Defaults are CS6's off its dialog: frame 34 (Pinwheel), vine `60 84 34`,
margin 14, size 20, arrangement 1, flower 2, flower colour `0 88 218`, flower
size 20, leaf 4, leaf colour `104 170 60`, leaf size 20, lines 15, thickness
29, angle 10, fade 0, invert off.

## Dialog

CS6's Picture Frame dialog has a frame thumbnail picker, a preview, and
Basic / Advanced option pages. That is too rich for the shared slot dialog, so
`PictureFrameDialog` is its own `QDialog`, built on `FilterParamControls` for
its option rows (Hidden frame slot, Slider, Choice, Color, Angle, CheckBox).

- **Frame picker.** A `QListWidget` of the 47 frames with a small thumbnail
  each, rendered once through `filter_thumbnail` at the default settings. The
  selection drives slot 0 through a `Hidden` control in the Basic group.
- **Preview.** A pad showing the current picture proxy with the frame drawn
  through `filter_thumbnail`; it follows every control change cheaply.
- **Basic page.** Vines colour, Margin, Size, Arrangement, Flower (None + 22),
  Flower colour, Flower size, Leaf (None + 23), Leaf colour, Leaf size.
- **Advanced page.** Number of Lines, Thickness, Angle, Fade, Invert.
- **Canvas preview.** With Preview on, `filter_preview` renders the whole
  layer on open (deferred a turn), on picker/control changes, and on slider
  release; a drag updates only the pad. Cancel discards the preview.
- **Whole-layer preview.** `picture-frame` joins
  `filter_preview_needs_whole_layer`, because its size and inset are fractions
  of the layer.
