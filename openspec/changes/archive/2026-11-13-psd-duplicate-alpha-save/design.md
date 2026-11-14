# Design

## Context

See `proposal.md` and the delta spec for the required behavior. The current save
path is:

- the reader splits a four-channel RGB source into a three-plane composite and
  one document extra channel (`crates/pictura-codec/src/read.rs`);
- the application keeps an RGB composite as four-plane RGBA for display, either
  by expanding the stored composite (`DocumentComposite::store` /
  `rgba_frame`) or by compositing the layer stack into tiled planes
  (`composite_tiled`);
- `save_view` (`crates/pictura-render/src/document_ops/mode/mod.rs`) is the
  normalization seam before the writer. It trims the display alpha only when
  `source_mode` is set;
- `write_psd` derives the header color count from `doc.composite.channels`
  (`crates/pictura-codec/src/write.rs`) and appends `doc.channels` after it.

## Goals / Non-Goals

**Goals:**

- Restore `header channels = mode color channels + document extra channels`
  for RGB saves whose display composite is RGBA and which carry extras.
- Keep the current four-channel write for an RGBA document with no extras.
- Fix all application save paths without a codec contract change.

**Non-Goals:**

- Distinguishing merged transparency from a user alpha channel through the
  PSD negative-layer-count convention; that modeling gap is separate.
- Changing the writer's treatment of constructed or converted documents that
  do not carry extras.

## Decisions

### Trim in `save_view`, not in the writer

`save_view` exists to hand the writer a document shaped for the format, and it
already drops the display alpha for a recorded source mode. The writer's
`composite.channels` is a display property when the document has extras, and
changing `write_psd` to infer the mode's color count would alter every direct
writer caller, including smart-object payloads. The issue's writer-side
alternative (skip an extra plane that duplicates the composite alpha) is
rejected because it only trims when the bytes match: the display alpha of a
layerless document is synthesized as 255, so an arbitrary source alpha would
still produce a fifth channel.

### Key the trim on `doc.channels`, not on layer or composite state

The bug appears in two states: a layerless document whose stored three-plane
composite was expanded to RGBA, and a layered document whose composite is
recomposited into tiled RGBA planes. Both carry their source alpha in
`doc.channels`, so the condition is "RGB, composite has four planes, and the
document has at least one extra channel". A document with no extras keeps the
RGBA merged image and its current four-channel output.

### Copy the color planes without materializing the tiled mirror

The trim copies the first three planes into a new three-plane buffer with
`Plane::read_into`. Reading the tiled `composite_tiled` plane through `as_ref()`
would first materialize and cache the whole four-plane mirror (~30 MB at
4934x1500) before the three-plane copy (~22 MB); `read_into` keeps the cost to
the 22 MB target buffer, acceptable for an effort-low fix. A plane view or a
writer-side color count would avoid even that copy but would widen the change;
the `ponytail:` comment names that ceiling.

## Risks / Trade-offs

- [A transparent layered RGB document that also has saved alpha channels loses
  its merged display alpha in the written merged image] -> Accepted: the
  requested output is color planes plus extras. Modeling merged transparency
  through the negative-layer-count convention is a separate capability gap.
- [A three-plane target buffer is allocated when extras exist] -> The
  `read_into` copy is ~22 MB at 4934x1500 (three planes), bounded by the canvas
  size and only on the save path; the tiled four-plane mirror is not
  materialized. Revisit with a plane view if save memory becomes a measured
  problem.
- [A 16/32-bit dirty layered save changes its header count] -> Intended: the
  count now matches a clean save, as `compositing/native-depth-save` requires.

A no-extras four-plane document already re-reads as color planes plus one extra
channel; this change neither alters nor codifies that pre-existing asymmetry.
