# Proposal

## Why

Issue #1107. A 16- or 32-bit layered PSD saved by Photoshop opens with the merged
image and an empty Layers panel. Photoshop stores those layers in a
document-level `Lr16` or `Lr32` tagged block and leaves the standard layer-info
subsection empty. The reader parses only the subsection, so it keeps the block as
opaque bytes and finds no layers.

## What Changes

- **Reader.** With an empty layer-info subsection, the block matching the document
  depth is parsed as layer info and removed from the preserved trailing blocks.
  `8B64` block signatures are accepted when walking those blocks.
- **Writer.** A 16- or 32-bit document with layers is saved in the same layout, so
  a file opened from Photoshop keeps that layout. Files saved by earlier builds,
  with layers in the subsection, still open.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `codec/psd-bit-depth`: where 16- and 32-bit layer info is read from and written to.

## Impact

`crates/pictura-codec/src/{read.rs,read/tagged_layers.rs,write.rs,filter_effects.rs,common.rs}`
and the depth fixtures. No API or model change.
