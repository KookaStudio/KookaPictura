# Design: vendor-raw-orf-compressed

## Module

`crates/pictura-codec/src/dng/orf_decode.rs`, ported from LibRaw 0.22.1
(`src/decoders/olympus14.cpp`, commit b860248a); `orf.rs` dispatches to it
and `orf_meta.rs` reads its parameters.

## Detection

The directory calls every ORF coding uncompressed, so the strips' total size
still picks the coding (#1471). One strip of fewer than 1.5 bytes per
photosite is the compressed stream. Its parameters are the ImageProcessing
tags `0x0640`-`0x0649` and `0x0650`-`0x0653`, each 0 when absent, as in
LibRaw; with none of them the file is refused as unsupported.

On all 23 compressed samples (raw.pixls.us and local; 17 bodies, E-410 to
OM-1 Mark II, two files at 14 bits) the parameters are the same apart from
`0x0653`, the stream's header length: 5 bytes at 12 bits, 6 at 14.

## Stream

The strip starts with a header: `0x0653` bytes holding a big-endian 1, then
two zero bytes. After it, one bit stream runs over the whole plane, read
most significant bit first from big-endian words, and continues past the
strip to the end of the file as LibRaw's reader does; reads past the file
yield zeros.

Each photosite's code is a sign bit, `0x0643` fine bits, `0x0640` raw low
bits, then a unary high part ended by a 1 bit, or after `0x0652` zeros an
escape holding the high part in `15 - wbits` bits, then `wbits` low bits.
`wbits` follows the bit length of the code two photosites back (the same
colour), with thresholds `0x0646`-`0x0651`. The value is then folded with a
fine context from earlier codes, and added to a prediction from the
same-colour neighbours two columns left (W), two rows up (N) and both (NW):
N, W, their mean, or `N + W - NW` by the gradients and the edge threshold
`1 << 0x0641`. Outside the first two rows or columns the neighbour is
`0x0642`.

## Bounds

LibRaw takes any parameter values. The port refuses, as unsupported, values
that would make a shift undefined (`0x0640` or `0x0643` above 16, `0x0641`
above 30, `0x0644` above 15, `0x0645` mod 32 above 30, `0x0651` above 15)
and a zero header length, which LibRaw also rejects. Every code holds at
least `2 + 0x0643 + 0x0640` bits, so a strip shorter than that over the
plane is invalid before any decode; a hostile small file cannot make the
decoder grind through a large plane of zeros.

LibRaw shifts 32-bit registers by 32 where all their bits are valid, which
is undefined in C and on x86 shifts by 0; the port's `shl` does the same,
explicitly.

## Levels

The maker note records black and SensorCalibration at 12 bits in 14-bit
files too (OM-1 Mark II 14-bit: black 255 and SensorCalibration 4095, where
LibRaw reports black 1020 and white 16380 and the darkest samples are near
920), so both are scaled by 4 when the valid bits are 14, as LibRaw does.

## Maker-note strays

The E-P1 and E-PL1 record an ImageProcessing entry (`0x1104`, 4608 bytes)
whose offset points before the file. The strict TIFF reader refused the
whole note, which the earlier size-based refusal had hidden. Maker-note
directories are now read with `Tiff::ifd_skipping_strays`, which drops such
entries; every other directory stays strict.

## Performance

The stream is sequential: one pass, one thread, a few dozen integer
operations per photosite, as in LibRaw. Release reads, normalization
included: 0.35 s for the E-M5 (16 MP), 0.50 s for the OM-1 Mark II (20 MP),
1.77 s for an 81 MP OM-3 high-resolution shot.
