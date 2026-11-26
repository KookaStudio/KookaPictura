# Proposal: Fujifilm-compressed RAF sensor data (#1620)

## Why

The RAF reader (#1473) refuses Fujifilm's compressed sensor data, which many
X-Trans and GFX bodies write by default (X-H1, X-T5, X-S10, X-E4, X-H2,
GFX 50S II, GFX100S in lossless or lossy mode). Those files cannot be opened.

## What changes

- RAF sensor data that starts with Fujifilm's compressed-data header is
  decoded, lossless and lossy, on X-Trans and Bayer sensors at 12, 14 and 16
  bits, into the same normalized `DngImage` as the uncompressed codings.
- Data shorter than its packed size without that header is still refused as
  unsupported.
- Compressed strips are bounded by the sensor section rather than by the
  recorded data length, which LibRaw notes some bodies (X-H2S) understate.

## Capabilities

- Modified: `codec/vendor-raw-containers`.

## Provenance

By owner decision, the decoder is an open, attributed port of LibRaw
0.22.1's `src/decoders/fuji_compressed.cpp` (LGPL-2.1 side of its
LGPL-2.1/CDDL-1.0 dual licence, distributed under GPL-3.0-or-later), as for
the CR3 decoder (#1474). It is not a clean-room implementation. The port is
checked against LibRaw's own decode, through rawpy, of raw.pixls.us sample
files kept outside the repository.
