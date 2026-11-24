# Design: Canon raw white stays at the precision ceiling (#1587)

## Measurement

Each Canon CR2 and CR3 in the owner's sample folders (70 files, ColorData
versions 15, -4, 16-19, 32-34, 48 and 64-67) was opened with rawpy (LibRaw)
as an instrument, not as an oracle of the reader. The raw photosite values
were histogrammed and compared with the ExifTool-reported
`NormalWhiteLevel` / `SpecularWhiteLevel`:

| File | ISO | Normal / Specular | At 16383 | Above Specular |
|---|---|---|---|---|
| EOS Rebel SL2 (CR2) | 100 | 11435 / 11892 | 1.96 % | 3.03 % |
| PowerShot G1 X Mark III (CR2) | 100 | 11435 / 11892 | 0.013 % | 0.023 % |
| EOS Kiss M | 125 | 11435 / 11892 | 4.41 % | 9.31 % |
| PowerShot V1 | 640 | 13135 / 13660 | 6.66 % | 11.96 % |
| EOS R10 | 1600 | 13035 / 14338 | 0.34 % | 0.43 % |

Every file with blown highlights piles them at 16383; none piles at its
Specular or Normal level. LibRaw's white is 16383 for all of them. The
recorded levels are therefore not the clip point of the stored data, and the
precision ceiling is the right white.

## Layout notes

For the record (not read by the codec): `NormalWhiteLevel` sits at word
0x30e (version 15), 0x569 (-4), 0x31c (16-19), 0x32a (32, 33), 0x280 (34,
48) and 0x294 (64-67), with `SpecularWhiteLevel` the next word. Versions 66
(EOS R6 Mark III) and 67 (EOS R8 Mark II) follow the version-64 layout;
ExifTool misreads 66 and does not decode 67.

## Guard

A synthetic file of each container carries colour data that records white
levels below most of its samples; its samples must equal those of the same
file without colour data. A corpus check on decoded samples alone cannot
tell the two normalizations apart (both pile clipped photosites at 65535),
so the measurement above is kept as a manual plan row.
