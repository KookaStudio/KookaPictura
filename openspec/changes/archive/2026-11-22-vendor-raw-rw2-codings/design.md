# Design: vendor-raw-rw2-codings

## Modules

`crates/pictura-codec/src/dng/rw2_decode.rs` is the port: one function per
coding plus the zero fill, under rawspeed's copyright notices.
`rw2.rs` chooses the coding, computes its input length and levels, and keeps
the plain codings. No new dependency.

## Codings (rawspeed names)

| RawFormat | rawspeed | Layout |
|---|---|---|
| 3 (no `0x0118`) / 4 | V4 | 14 photosites per 16 bytes, predicted per colour with a 2-bit shift per three; 0x4000-byte blocks swapped at 0x1FF8 when `RawDataOffset` is present, unswapped otherwise |
| 5 | V5 | the plain 16-byte groups (10 × 12 or 9 × 14 bits) inside swapped 0x4000-byte blocks |
| 6 | V6 | 16-byte packets of 14 (12-bit) or 11 (14-bit) photosites, base and pair-wise prediction, less 15 |
| 7 | V7 | the plain 14-bit groups #1472 already reads |
| 8 | V8 | strips at offsets from tags `0x0044`–`0x0048`, Huffman-coded differences (table `0x0040`) predicted per 2 × 2 block |

RawFormat 8 refuses a non-identity gamma curve (`0x0039`, `0x003A`), a shift
(`0x0041`) or a clip (`0x003B`) other than rawspeed's observed values, and
strips that do not tile the plane. rawspeed starts both greens of a strip
from tag `0x003E`; the port keeps that.

## Levels

RawFormat 3 and 4 add 15 to the black tags (rawspeed and LibRaw agree, and
the data's floor is 16 on the FZ28 and G1). rawspeed also adds it to the
plain codings of files without `RawDataOffset`, but an FZ30's data reaches 1,
so #1472's levels are kept there. The white stays the green linearity limit,
within the coding's range (16 bits for RawFormat 8).

## Zero photosites

rawspeed lists RawFormat 3 and 4's zeros as bad pixels and replaces each by
the distance-weighted mean of the nearest good photosite of its colour left,
right, up and down. The port finds those by sweeping each affected row and
column once, so the cost stays linear in the plane however many zeros it
holds.

## Verification

The nine samples (L10, G1, FZ28, GX9, G9, GH5S, S1, GH6, G9 II) were decoded
before the levels and compared with LibRaw 0.22.1's `raw_image` through
rawpy: every photosite matches, except the zeros, which LibRaw fills later.
Half-size renders of all twelve samples are intact.

## Patents (OVR-004)

Google Patents, 2026-11-22, status only; not a legal conclusion and not an
assessment of whether any claim reads on these codings or on decoding:

| Patent | Subject | Status listed |
|---|---|---|
| US8090209B2 (WO2010035433A1, JP5208218B2) | predictive coding into fixed-width blocks, Panasonic, now Nuvoton | active, anticipated expiry 2029-09-16 |
| US9106250B2 (WO2011061885A1) | fixed-length RAW encoding for cameras, Panasonic, now Nuvoton | active, adjusted expiry 2032-02-08 |
| WO2007013335A1 | same-colour difference coding with a chosen quantization width, Matsushita | PCT ceased; national grants not identified |

Shipping is the owner's call.
