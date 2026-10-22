# Design — Keep the stroke band at the selection's edge, and the fill's transparency

## The one idea

All three fixes are the same mistake in different clothes: **the engine has
one way of treating a pixel's alpha, and every caller that needs a different
one works around it.**

- The stroke works around the dilate's cascade by accident, and nobody
  noticed because the workaround's symptom looks like a feature.
- Preserve Transparency works around the engine's blend by building a coverage
  mask in the app, which can express *where* to paint but not *what happens to
  the alpha*.
- The gradient's ramp alpha is dropped because the only blend the fill has is
  a source-over, and a source-over cannot cut a hole.

So the engine grows one explicit knob — the alpha policy — and the callers
that want a different behaviour ask for it by name. That is the whole design.
No new engine concept, no new dependency, no new file.

## The alpha policy

```rust
/// What the fill does with the pixels' transparency.
pub enum Alpha {
    /// The source over the backdrop: the backdrop's alpha survives.
    Blend,
    /// The destination's alpha survives, and its empty pixels are left alone.
    Keep,
    /// The source's alpha becomes the pixel's alpha.
    Replace,
}
```

`Alpha` lives in `pictura-paint` and is `pub`, because `bucket::fill` is
`pub` and takes it. `fill_planes` — the one function that writes an alpha —
takes it and picks:

| Policy | The pixel's alpha becomes | The empty pixels |
|---|---|---|
| `Blend` | the blend's own result (`out_a = src_a + da * (1 - src_a)`) | filled |
| `Keep` | the destination's, untouched | **skipped** |
| `Replace` | the source colour's alpha | filled |

`Replace` reads the source colour's alpha directly rather than a value the
blend returns, so the `Blend` signature does not change: `blend_pixel` and
`blend_native` keep their shape, and the policy decides the alpha in the
caller, where the colour and the strength are both in hand.

### The lock outranks the caller

`fill_layer` computes the layer's transparency lock today and passes it down
as `keep_alpha`. With the policy, the lock *forces* `Keep`: a
transparency-locked layer keeps every pixel's alpha whatever the caller
asked for. The lock is the user's explicit instruction about the layer's
transparency, so it wins over a per-command option. `Keep` also skips the
empty pixels, which is what "transparent areas are not filled" means in the
corpus — and it is what the layer's lock already did.

### The three callers

- **`Edit > Fill`** (`pictura-app`): `Keep` when the dialog's Preserve
  Transparency is set, `Blend` otherwise. The app's coverage-mask workaround
  is deleted. The mask could only say *where* to paint, so it could not keep
  a half transparent pixel's alpha; `Keep` keeps it, and skips the empty
  pixels exactly as the mask did.
- **`Edit > Stroke`, the Paint Bucket, the shape and path fills, the mask
  fill**: `Blend`, unchanged.
- **The Gradient** (`pictura-paint`): `Replace` when the Transparency option
  is on *and* the mode is Normal, `Blend` otherwise.

### Why the gradient only replaces the alpha in Normal

In Photoshop the Gradient tool replaces the pixels' alpha with the ramp's —
it is a fill, not a brush. In Multiply, Behind and the rest the blend governs
the alpha, as it does everywhere else in the engine: a Multiply gradient over
an opaque layer leaves it opaque, and the ramp's transparency does not punch
a hole through it. So the gradient picks the policy by its mode, and the
engine stays mode-agnostic: no "if mode == Multiply" branch inside the fill.

### Why the coverage and the opacity stay merged

`fill_planes` multiplies the source's coverage by the selection's coverage,
and the Paint Bucket folds the dialog's opacity into the source's coverage
before it gets there. `Replace` takes the source colour's alpha as it is, so
a gradient drawn at 50 % opacity over an opaque layer leaves the layer
opaque — which is what Photoshop does — while a *feathered selection* does not
feather the gradient's alpha. Un-merging the coverage and the opacity would
fix the second case, and it is a larger change than the bug warrants:

```rust
// ponytail: a feathered selection does not feather a gradient's alpha; the
// source's coverage and its opacity arrive merged, and un-merging them is a
// larger change than the gap.
```

## The dilate and the erode

`dilate` and `erode` are separable min/max filters: a horizontal pass, then a
vertical one. The vertical pass reads the horizontal pass's output and writes
a buffer of its own. The horizontal pass read and wrote the *same* buffer:

```rust
let mut out = mask.to_vec();
for y in 0..h {
    for x in 0..w {
        let mut m = 0u8;
        for k in -r..=r {
            let nx = (x + k).clamp(0, w - 1);
            m = m.max(out[(y * w + nx) as usize]);   // reads what the loop just wrote
        }
        out[(y * w + x) as usize] = m;               // and writes it
    }
}
```

The value at `x` is the max over a window that reaches to the right, and the
window of the *next* pixel includes the value this one just wrote. The
dilation therefore cascades rightward: a selection's dilation runs to the end
of the row. The band is `dilated - selection`, so the band's right arm
stretched to the canvas's edge, and the band's top and bottom rows stretched
with it.

The fix is the vertical pass's shape: read the source mask, write a buffer of
your own.

```rust
let mut out = vec![0u8; mask.len()];
for y in 0..h {
    for x in 0..w {
        let mut m = 0u8;
        for k in -r..=r {
            let nx = (x + k).clamp(0, w - 1);
            m = m.max(mask[(y * w + nx) as usize]);
        }
        out[(y * w + x) as usize] = m;
    }
}
```

`erode` gets the same treatment: it reads the mask and writes its own
buffer, so the min filter no longer cascades either.

The filters are O(w · h · r) as they were; nothing about their cost changes.

## The 8-bit bytes do not move

`Alpha::Blend` is what every fill asked for before, and it is what the
policy defaults to. The two paths that change behaviour are the ones that
were wrong:

- Preserve Transparency on a **fully** transparent pixel: the mask skipped it
  and `Keep` skips it. Same result.
- Preserve Transparency on a **partly** transparent pixel: the mask filled it
  at full strength and its alpha became 255; `Keep` leaves its alpha alone.
  This is the fix.

So `an_eight_bit_fill_is_byte_identical` still holds for every fill that does
not ask for `Keep`, and the existing fill suite is unchanged.

## The tests that would have caught these

Each of the three bugs survived because the assertions sat where the bug did
not reach. The new tests sit where it does:

- The band's far end: the 8×1 row's last pixel, and a 24×24 canvas whose
  selection has room to its right.
- The half transparent pixel: a layer with an alpha of 128, filled with
  Preserve Transparency, at 8 bits and at 16 bits.
- The ramp's transparent end: a gradient whose far stop is transparent, drawn
  at 8 bits and at 16 bits.

## What is not in here

- The Paint Bucket's flood still matches its region at 8-bit precision.
- The Gradient's ramp alpha is not replaced in the other brush modes.
- A feathered selection does not feather a gradient's alpha.
- The 8-bit-only tools — the Brush, the Eraser, the filters — still write only
  the 8-bit channels; that is #490's class.
