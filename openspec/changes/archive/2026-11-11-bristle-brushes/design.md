# Design: bristle-brushes

## Model

`pictura-paint::bristle` owns the vocabulary:

- `BristleShape { Round, RoundFan, RoundCurve, Flat, FlatFan, FlatCurve }`.
  The six shapes are the documented round/flat × plain/fan/curve families; the
  exact CS6 enumeration is unpublished, so the list and the index order are a
  project choice.
- `BristleTip { shape, bristles, length, thickness, stiffness }`, all percents
  in `0..=100` and `Copy`. The field defaults (Bristles 50, Length 50,
  Thickness 50, Stiffness 50) are project defaults, not verified CS6 values.
- `BristleTrack { root, tip, half_width }` in dab-local coordinates. Tracks are
  rebuilt per dab from the tip, the dab diameter, the stroke tangent, and a
  stable per-dab seed.

`StrokeConfig::bristle` is `Option<BristleTip>`; `None` is every existing brush
and keeps the old raster bytes exactly.

## Geometry

For bristle index `i` of `count`, `t = 2i/(count-1) - 1` (0 when count is 1).
The shape profile returns a lateral root offset, an axial root offset, and a
fan angle:

- Round family: roots on an arc, so the union's outer edge is round.
- Flat family: roots on a line, so the union's outer edge is chiselled.
- Fan variants add an outward fan angle to each track's direction.
- Curve variants bend the root line into an arc.

The track runs from the root along the stroke direction rotated by the fan
angle. Stiffness controls the bend: low stiffness moves the track tip inward
and back along the stroke, collapsing the shape; stiffness 100 leaves the
radial line. The per-track seed adds a small deterministic length and lateral
wobble. Count is `1 + bristles * 63 / 100` (1–64), so Bristles 0 still paints
one bristle.

Track half-width is `radius * (0.02 + thickness / 100 * 0.08)`, tapering to a
third at the tip. A pixel's coverage is the maximum capsule coverage over the
tracks, with a one-pixel antialias band. Because a track is narrower than the
gap between roots at normal densities, a cross-section resolves into separate
streaks.

The mark reach is `radius + track_length`, where
`track_length = radius * (0.6 + length / 100 * 1.4)`. The stroke's saved-pixel
box, raster bounds, and x-span use that reach; off-canvas tracks are clipped by
the existing bounding box.

## Raster path

`DabRaster` gains an optional `&BristleDab`. When present it asks the bristle
dab for coverage and a full-width x-span; otherwise it keeps the hoisted
`TipParams` path. `PlacedDab` carries the stable seed and the effective tip, so
a jittered size/angle still builds the same bristle family. Dynamics continue
to place, scatter, and colour the dab before the bristle raster runs.

The preview and the preset thumbnails go through `paint_stroke`, so they pick
up the bristle raster automatically. The reduced-level live preview builds one
`BristleDab` per sample and uses the tip angle as its direction; the exact
committed stroke uses the path tangent. The preview is thrown away, so this
approximation does not affect committed pixels.

## Mixer reservoir

`MixerBrush` keeps a `Vec<[u8; 4]>` of per-bristle reservoirs, seeded from the
incoming brush reservoir when the tip's count or shape changes. A bristle dab
uses one `BristleDab`; the pickup is the tip-weighted mean of the whole dab,
each bristle reservoir lerps toward it by `wet * mix * PICKUP_RATE` with a
stable per-bristle factor, and each pixel deposits through the strongest track's
reservoir. `reservoir()` returns the per-bristle mean, so the bridge carries the
brush into the next stroke. Non-bristle mixer dabs keep the old single-
reservoir path byte for byte.

## Bridge, panel, persistence

`PaintTip` gains `bristle_enabled`, `bristle_shape`, `bristle_density`,
`bristle_length`, `bristle_thickness`, and `bristle_stiffness` at the end of the
struct, so existing C++ brace initializers keep compiling. `tip_config` turns
them into `StrokeConfig::bristle`.

`PaintOptions` gains the same six values. The Brush Tip Shape page adds a
Bristle Tip checkbox, a Shape combo, and four numeric fields; Hardness and
Roundness are disabled while a bristle tip is active because a bristle tip has
no hardness. All controls have object names for Qt Test. The values join
`PaintFields`, so the existing session save/restore and preset paths carry them.

The preset picker gains `Bristle Round 30` and `Bristle Flat 40` before the four
issue-#731 brushes. They are approximations, not Photoshop preset data.

## GPU

`gpu_paint_mode` declines a stroke whose `cfg.bristle` is set, exactly as it
already declines grain, Wet Edges, pressure overrides, and dynamics. The CPU
bristle raster is the oracle.

## Determinism and limits

Track seeds derive from the stroke's fixed seed and the dab index, so a replayed
stroke paints the same pixels. The raster keeps the existing tiled parallel
split; each tile only reads its own pixels. A 5000 px tip with 64 bristles is
expensive; the raster prunes to the dab's reach but does not add a separate
profile cache. That is recorded as a `ponytail:` ceiling rather than claimed as
CS6 performance.

Bristle tips are wired to the Brush/Pencil Paint path and the Mixer Brush. The
per-dab retouch engines keep their round `tip_coverage` until a later change;
that is a recorded ceiling, not a claim of full coverage.
