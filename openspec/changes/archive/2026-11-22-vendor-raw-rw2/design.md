# Design: vendor-raw-rw2

## Modules

`crates/pictura-codec/src/dng/rw2.rs` holds the signature, the three packings
and the reader. `vendor.rs` gains the sniff and the dispatch. `Tiff::view`
(#1471) reads the directory behind the RW2 magic.

## Tags (IFD0)

| Tag | Value | Use |
|---|---|---|
| `0x0002`, `0x0003` | sensor width, height | the stored plane |
| `0x0004`-`0x0007` | top, left, bottom, right | active area, the last two exclusive |
| `0x0009` | 1 RGGB, 2 GRBG, 3 GBRG, 4 BGGR | CFA from the sensor's corner |
| `0x000A` | bits | checked against the coding |
| `0x000B`, `0x002D` | compression, RawFormat | choose the coding |
| `0x000E`-`0x0010` | linearity limit R, G, B | white, the green one |
| `0x001C`-`0x001E` | black R, G, B | black per colour |
| `0x0024`-`0x0026` | white-balance levels | as-shot neutral |
| `0x002F`-`0x0032` | top, left, bottom, right | crop in sensor coordinates |
| `0x0118`, else `0x0111` | data offset | `0x0111` is `0xFFFFFFFF` beside `0x0118` |
| `0x0117` | data length | must equal the coding's size |

## Sensor data

432 files from raw.pixls.us, 2026-11-22:

| Compression, RawFormat | Data | Files | Result |
|---|---|---|---|
| 34828 or 34830, none | left-aligned little-endian 16-bit words | 7 (FZ30, FZ50, L1, LX1, LX2) | read |
| 34316, 2 | 16-byte groups: ten 12-bit samples from the low bit up, then padding | 2 (FZ8, FZ18) | read |
| 34316, 7 | 16-byte groups: nine 14-bit samples from the low bit up, then padding | 3 (S5) | read |
| any, 5 | full depth, no plain order fits | 10 | refused (#1541) |
| any, 3, 4, 6, 8 | fewer bytes than the bit depth needs | 410 | refused (#1541) |

The packings were found by trying plain byte and bit orders and keeping the
smoothest, then confirmed by rendering beside camera previews where present.
Padding bits are not zero in the FZ8 and FZ18 files and are ignored.

## Levels

The words hold 12-bit levels shifted left by four, also on the DMC-L1, which
declares 14 bits and uses two more low bits while its tags still say 4095.
So a word file's levels are the tags times 16. The packed codings use the
tags as they stand.

## CFA

The code counts from the sensor's corner. Borders are often odd (top 7,
left 15), so the reported pattern is shifted to the active area's corner.

## Decisions

- **Owner (#1472)**: plain packings only, the rest refused and tracked in
  #1541; decode only, no proxy matrices, colour tracked in #1542; #1472 stays
  open.
- **Filters**: not listed in the Open dialog while none of these files opens.
  "All files" still reaches the raw open by signature.
- **Known limit**: old bodies stop short of their white tag (FZ8 and FZ18 at
  3967, FZ50 at 3971, tag 4095). The tag is used; #1542 checks highlights
  once there is colour.
- **Ponytails**: one white for all colours (the green limit); a crop outside
  the borders falls back to the active area instead of being clipped.
