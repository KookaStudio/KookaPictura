# Proposal: Olympus-compressed ORF sensor data (#1535)

## Why

The ORF reader (#1471) refuses Olympus's compressed sensor data, which nearly
every Olympus and OM System body from the E-410 (2007) to the OM-5 Mark II
writes in its normal mode: 90 of the 112 ORF/ORI files on raw.pixls.us, from
51 models, including the 14-bit high-resolution shots of the OM-1 Mark II.
Those files cannot be opened.

## What changes

- An ORF whose one strip is shorter than the 12-bit packed size is decoded
  as Olympus's compressed stream, at 12 and 14 valid bits, into the same
  normalized `DngImage` as the uncompressed codings.
- The stream's coding comes from the maker note's ImageProcessing tags
  `0x0640`-`0x0653`; a compressed ORF without them is refused as
  unsupported, and one whose parameters fall outside the bounds the decoder
  checks is refused too.
- 14-bit ORFs take their black and white, recorded at 12 bits, scaled by 4.

## Capabilities

- Modified: `codec/vendor-raw-containers`.

## Provenance

By owner decision, the decoder is an open, attributed port of LibRaw
0.22.1's `src/decoders/olympus14.cpp` (LGPL-2.1 side of its
LGPL-2.1/CDDL-1.0 dual licence, distributed under GPL-3.0-or-later), as for
the CR3 (#1474) and compressed RAF (#1620) decoders. It is not a clean-room
implementation: the #1471 session saw fragments of third-party decoders of
this coding, and a separate model-written attempt stopped on recognising
training memory of one. The port is checked against LibRaw's own decode,
through rawpy, of raw.pixls.us sample files kept outside the repository.

## Patents

`OVR-004` asks for a patent check per codec. A Google Patents status search
on 2026-11-22 (not a legal conclusion and not a complete search) listed
US7715639B2 as active with an adjusted expiration of 2029-02-01, and
US7333035B2, US7259698B2 and JP2006211155A (JP4434974B2) as expired. Whether
the active patent covers ORF files or decoding them has not been assessed;
the owner decides before merge.
