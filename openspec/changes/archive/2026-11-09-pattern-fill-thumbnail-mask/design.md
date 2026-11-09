# Design — Photoshop pattern slot layout

## Context

A `Pattern` in the `Patt`/`Pat2`/`Pat3` tagged blocks ends in a
`VirtualMemoryArrayList`: a version, a rectangle, a declared `num_channels`, and
`num_channels + 2` `VirtualMemoryArray` slots. Photoshop writes **24 colour
slots and two alpha slots for every pattern**, regardless of the image mode, and
writes pixels only into the slots the pattern uses. An RGB pattern is normally
slots 0–2 plus the last slot (25); the other 21 colour slots are unwritten.
psd-tools derives the colour count from the written slots in the leading region
(`get_pattern_color_channels`), not from the image mode.

The current decoder treats `num_channels` as the mode's plane count and rejects
any other value, so a real Photoshop pattern is skipped and the compositor and
the Layers thumbnail both draw the 50 % grey placeholder.

## Goals

- Decode a Photoshop-authored Grayscale or RGB pattern with 24 colour slots.
- Keep the existing Grayscale/RGB scope and placeholder fallback for other
  modes.
- Keep the decoded work bounded: a 24-slot pattern still decodes at most the
  mode's colour planes plus the two alpha slots.
- Preserve the existing malformed/oversized-file behaviour.

## Non-goals

- Indexed and CMYK pattern decoding (they still fall back to the placeholder).
- Decoding colour slots that the pattern does not use.
- A new public pattern API; `PatternPixels` is unchanged.

## Decisions

### The declared count is the colour region, not the mode width

`parse_channel_list` reads `num_channels` as the number of leading colour slots
and requires it to be at least the mode's colour-plane count. It then reads
`num_channels + 2` channel arrays. The first `expected_color` slots are decoded
as the colour planes and must all be written; the two trailing slots are decoded
as the alpha region and the last written one wins. Every slot in between is
read past without decompressing its plane.

This mirrors psd-tools' slot-keyed split while keeping this decoder's stricter
contract (all colour planes present) and its existing per-channel pixel cap.

### Bound the slot count at Photoshop's 24

The format's maximum channel count is larger than a pattern ever uses. The
decoder accepts at most 24 colour slots (plus two alpha slots); a larger
declared count skips the pattern. This bounds the metadata walk and keeps a
crafted file from asking for an arbitrary number of channel headers.

### The fixture mirrors Photoshop but is generated in-repo

A Photoshop-authored sample supplied with the issue confirms the 24-slot layout,
but the repository's provenance rules forbid committing Adobe-authored files or
Adobe preset patterns. `scripts/generate-fixtures.py` therefore authors a small
8x8 PSD with the same pattern slot layout (24 colour slots, three written RGB
planes, a trailing alpha) and a painted diagonal layer mask. The codec test
asserts the decoded pattern id, size, and pixels plus the mask's presence and
coverage; the Qt test opens it and asserts the Layers row's pattern thumbnail is
not the grey placeholder and that the mask thumbnail is non-null.
