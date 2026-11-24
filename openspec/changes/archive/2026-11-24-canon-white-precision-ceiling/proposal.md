# Proposal: Canon raw white stays at the precision ceiling (#1587)

## Why

#1587 asked the CR2 and CR3 readers to normalize to the white levels Canon
records in the maker note's colour data (tag `0x4001`: `NormalWhiteLevel`,
`SpecularWhiteLevel`) instead of the frame's precision ceiling, on the
assumption that the sensor clips there. Measured on 70 Canon files, it does
not: blown highlights pile up at the 14-bit ceiling 16383, well above the
recorded levels (EOS Rebel SL2 at ISO 100: Specular 11892, 1.96 % of
photosites at 16383 and 3.0 % above Specular; PowerShot V1 at ISO 640: 6.7 %
at 16383). LibRaw also takes 16383 as white for every one of these files.
Normalizing to Specular would flatten about half a stop of recorded
highlight detail to white.

## Owner decisions (2026-11-24)

- **Record and guard.** Keep the precision ceiling; record the finding in
  the readers and the spec, and add a test that colour data recording lower
  white levels leaves the samples unchanged. The change closes #1587.
- **No parsing.** The readers do not read the colour data's white levels;
  nothing would use them.

## What changes

- `codec/vendor-raw-containers`: the CR2 and CR3 normalization requirements
  state that the colour data's white levels do not set the white, each with
  a scenario.
- `cr2.rs` / `cr3.rs`: the `ponytail:` comment's white half becomes a
  recorded finding; the black half stays a ponytail.
- Tests in `raw_cr2.rs` and `raw_cr3.rs`.

## Capabilities

- Modified: `codec/vendor-raw-containers`.
