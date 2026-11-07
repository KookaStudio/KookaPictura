# Design

## Data model

`ChannelMixerParams` keeps its RGB rows and gains the CMYK rows; the document
mode chooses which set is applied. The block bytes carry both interpretations,
so decode stays mode-independent and only the editor and the apply sites take a
mode.

```rust
pub struct ChannelMixerParams {
    pub monochrome: bool,
    pub red: [f64; 3],
    pub green: [f64; 3],
    pub blue: [f64; 3],
    pub constant: [f64; 3],
    /// C/M/Y/K output rows over C/M/Y/K sources.
    pub ink: [[f64; 4]; 4],
    /// Per-output ink constant.
    pub ink_constant: [f64; 4],
}
```

`Default` is the RGB identity plus the CMYK identity, so existing constructors
add `..Default::default()`.

## `mixr` layout

Each row is 10 bytes: four big-endian `i16` sources, then the constant, i.e.
five shorts. The existing RGB form reads three sources, skips the fourth, and
reads the constant; ag-psd skips the same two bytes. A CMYK form uses the fourth
slot as the K source:

```
u16 version = 1
u16 monochrome
row 0: C source C, M, Y, K, constant
row 1: M source C, M, Y, K, constant
row 2: Y source C, M, Y, K, constant
row 3: K source C, M, Y, K, constant
```

Non-monochrome RGB writes red/green/blue/gray with the fourth source zero, as
today. Monochrome writes the gray row followed by 30 zero bytes, as today.

`decode_adjustment` reads all four rows and fills both views:

- RGB rows from rows 0..3's first three sources and constants.
- `ink` from all four rows' four sources and constants.

The CMYK interpretation is inferred: psd-tools reads only five shorts, ag-psd
skips the fourth source, and no library models a CMYK `mixr`. The layout is
tested by round-trip and hand-built bytes; the spec marks it inferred.

`encode_channel_mixer_ink(ink, ink_constant)` writes the CMYK form and clamps
each value to `-200..=200`. `encode_channel_mixer` keeps the RGB signature and
form.

## CMYK kernel

The engine works in unit ink: `0` is no ink (white), `1` is full ink (black).
For each output `c` and pixel:

```
out[c] = clamp(sum_s(w[c][s] / 100 * in[s]) - constant[c] / 100, 0, 1)
```

A negative constant therefore adds ink (black) and a positive constant removes
ink (white), matching the spec's black/white-channel wording. The identity rows
`[100,0,0,0]`, `[0,100,0,0]`, `[0,0,100,0]`, `[0,0,0,100]` are a no-op.
Weights and constants are validated to `-200..=200`; a bad value is
`AdjustError::InvalidParams` and the caller is a no-op.

`pictura_adjust::apply_channel_mixer_ink` operates on four planar `f32` planes.
It is separate from `apply` because `apply`'s buffer is RGB.

## RGB ↔ unit CMYK

`pictura-render::channel_mixer_cmyk` owns the conversion.

- **Profile-aware.** When the document is 8-bit and carries a four-channel CMYK
  ICC profile (resource 1039), convert the planar RGB bytes with
  `rgb_to_cmyk_with_profile`, map the device bytes to unit ink (`v / 255`), run
  the kernel, map back, and convert with `cmyk_to_rgb_with_profile`. The profile
  conversion is 8-bit by construction, so this path is used only for 8-bit
  documents.
- **GCR fallback.** Otherwise convert each pixel in f32:

  ```
  k = 1 - max(r, g, b)
  inv = 1 - k
  c = (1 - r - k) / inv, m = ..., y = ...
  rgb = ((1 - c) * (1 - k), (1 - m) * (1 - k), (1 - y) * (1 - k))
  ```

  This is an exact round trip and stays in f32, so a 16/32-bit document is not
  quantized to 8 bits. A profile-free 8-bit document uses the same path.

A document with no profile uses the fallback; a profile that fails to parse is
treated as absent.

## Apply sites

- **Adjustment layer (compositor).** `composite_adjustment` has the `Document`.
  When the adjustment is a non-monochrome `ChannelMixer` and
  `document_color_mode(doc) == Cmyk`, it runs the CMYK path on the canvas, then
  gates the adjusted canvas through the layer's mask/opacity/blend exactly like
  the RGB path. A monochrome mixer always uses the RGB path.
- **Destructive (Image > Adjustments).** `apply_adjustment_region` gains a
  `CmykMix` context (profile and eight-bit flag) supplied by the app from the
  document. The op converts the clip's RGB planes, applies the ink kernel, and
  converts back before the existing coverage/mask write-back. The destructive
  path works on the 8-bit working planes, so it uses the profile-aware path
  when available and the GCR otherwise.
- **GPU.** Channel Mixer is not GPU-eligible today; the CMYK path is CPU-only
  and falls back exactly as the RGB path does.

## Descriptors and defaults

`adjustment_editor_in_mode(data, cmyk)` and
`set_adjustment_param_in_mode(data, key, value, cmyk)` take the document mode.
For CMYK:

- groups are `Cyan`, `Magenta`, `Yellow`, `Black`;
- source keys are `{output}.cyan`, `.magenta`, `.yellow`, `.black`;
- fields are `I16(4 + output * 10 + source * 2)` and the constant at `+ 8`;
- no Monochrome check.

For RGB the page is today's plus `check("monochrome", "Monochrome", U16(2),
false)`. `set_adjustment_param_in_mode` special-cases `monochrome`:

- **On:** if the RGB rows are the identity, set the Red row to `[40, 40, 20]`;
  otherwise keep the current Red row. Set `monochrome = true`, restore the
  Green/Blue identity rows, and zero their constants. Re-encode the RGB form.
- **Off:** keep the Red row as the Gray mix, set `monochrome = false`, restore
  the Green/Blue identity rows, and zero their constants. Re-encode the RGB
  form.

The PSD block cannot carry the hidden per-output rows, so the rule is explicit
and tested.

`default_adjustment_block_in_mode(kind, fg, bg, cmyk)` returns the CMYK identity
for a CMYK document. The app bridge passes `image_mode(view) == "cmyk"`.

## UI

`ChannelMixerDialog` builds its source rows from the mode: Red/Green/Blue with
their ramps, or Cyan/Magenta/Yellow/Black with ink ramps. The Output Channel
combo comes from the descriptor page, so it shows Gray when monochrome and
C/M/Y/K in a CMYK document. The Monochrome checkbox is added for RGB only and
is named `monochrome` for the tests. Toggling it calls
`set_adjustment_param("monochrome", …)`, then reloads the combo items and the
current row.

The Properties panel uses the same descriptor page; its bridge already has the
view, so it passes the document mode. A monochrome toggle changes the page
signature, so the generic controls rebuild the group menu and show Gray.

## Limits

- CMYK mixing is unavailable for Grayscale, Lab, Indexed, Bitmap, and
  Multichannel, matching the spec's mode table.
- A mixer authored in one mode is not converted when the document changes mode;
  it is applied in the new mode's interpretation of the same bytes. Documented
  as a ceiling.
- The CMYK `mixr` layout is inferred. The acceptance test is round-trip and
  hand-computed; a Photoshop-authored CMYK fixture is the follow-up that would
  confirm it.
