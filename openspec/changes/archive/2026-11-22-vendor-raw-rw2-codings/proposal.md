# Proposal: vendor raw containers — Panasonic RW2 compressed codings (#1541)

## Why

The RW2 reader (#1472) decodes only the plain packings, 12 of 432 sample
files. The other 420 (RawFormat 3, 4, 5, 6 and 8: every Panasonic and Leica
body from 2008 on) are refused. #1541 was blocked on provenance: the AI
authoring model has training exposure to open-source decoders of these
codings, so a decoder it wrote could not count as independent.

## Owner decisions (2026-11-22)

- The provenance rule changes for this format only: the codings are ported
  openly from darktable's rawspeed (LGPL-2.1-or-later, compatible with the
  project's GPL-3.0-or-later), with its copyright notices and an entry in
  `assets/PROVENANCE.md`. It is a port, not a clean-room implementation.
- All refused codings ship in one change, closing #1541.
- A patent status search is recorded (OVR-004); it is not a legal conclusion.

## What changes

- RawFormat 3 and 4 (predicted 12-bit packets), 5 (packed groups in swapped
  blocks), 6 (paired 12- and 14-bit packets) and 8 (Huffman-coded strips) are
  decoded. Only unknown RawFormats and bit depths are refused.
- RawFormat 3 and 4 add rawspeed's 15 to the black tags and fill zero
  photosites from their neighbours, as rawspeed does.
- Decode only, as before: these bodies still have no colour (#1542), so File ▸
  Open refuses them naming the camera.

## Capabilities

- Modified: `codec/vendor-raw-containers`.

## Provenance

Ported from rawspeed commit 718eae230df9ebb1750755b775e52185f2342a46
(`PanasonicV4`–`V8` decompressors, `Rw2Decoder`, `fixBadPixel`). The port was
checked bit-exact against LibRaw 0.22.1's unpacked sensor data on nine sample
files of every coding (raw.pixls.us, kept outside the repository). The plain
codings and the container keep their #1472 provenance.
