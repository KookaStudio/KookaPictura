# Design

## D1. Which commits defer

`Document::composite_is_authoritative` already says whether the stored
`composite` is the picture: a layerless document, a non-RGB mode, a non-8-bit
depth, or a document with source planes. Only those need the worker's region
composite because a reader or the display may crop the stored frame.

A layered RGB 8-bit document keeps no stored composite. `refresh_display_rect`
already leaves a region above `REPAIR_PIXELS` stale on a large document instead
of compositing it; the visible tiles rebuild from the moved layers. Deferring
that move to a worker only duplicates the work and occupies every core while
the GUI repaints.

`should_defer_move` therefore returns false for a non-authoritative document
unless a test delay is set. The delay seam forces the old path so the
pending-composite tests keep their coverage.

## D2. Dropping the stale frame

A document can still carry a composite from an earlier state. The lazy tile
path drops it before the history capture, matching what `DocumentComposite`'s
own store/patch does for a non-authoritative document. History then captures
layer tiles rather than an old frame.

## D3. Release behaviour

The commit returns synchronously and reports `moveCommitted`. The drag preview
drops at once. Dirty display tiles are marked stale; the canvas asks for the
tiles it presents and their worker rebuilds them. Off-screen tiles stay stale
until a later pan or zoom.

This trades a brief stale/coarse frame after release for no multi-second GUI
stall. The exact tiles are byte-identical to a whole rebuild, as the existing
tile tests require.

## D4. Scope

The change covers Move and transform commits, both of which share
`should_defer_move`. A large transform still runs its resampling job; only its
post-transform region composite becomes lazy for a non-authoritative document.
