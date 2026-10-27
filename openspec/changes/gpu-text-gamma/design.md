# Design

## Context

See `proposal.md` — Why. The relevant current state:

- The CPU oracle applies the curve in `blend_into_row`
  (`crates/pictura-render/src/composite_rows.rs:107`): `content = source alpha ×
  layer opacity × Fill`, then `curve.apply(content)`, then `content × mask ×
  gate`. The same helper composites pixel layers, isolated groups, and
  adjustment layers.
- `text_gamma::current_curve()` returns a cached 4097-sample `f32` table per
  gamma (`text_gamma.rs:43-69`); `TextCurve::apply` linearly interpolates it.
- `check_supported` (`gpu/mod.rs:486`) declines a type layer whenever the gamma
  is on, before dispatch. `ContentRoute::TypeSource` already declines a
  proxy-less type layer at `gpu/mod.rs:473`.
- The blend shader computes `as_ = s.a × opacity × fill × mask_a` at
  `gpu/shader.rs:354`; the adjustment branch does the same at `:338`.

## Goals / Non-Goals

**Goals:**

- A proxied type layer with the gamma on composites on the GPU and stays within
  ±1 LSB of the CPU oracle at γ 1.00, 1.45, 2.20, and off, with opacity, Fill <
  255, and a mask.
- The remaining GPU text decline is attributable: proxy-less type only.
- No Rust type/signature or CPU output change; the metrics reason key is
  renamed.

**Non-Goals:**

- Type layers that carry a rendering layer effect (still
  `UnsupportedLayerEffect`; the layer-source cache in #969 is the prerequisite).
- Proxy-less live type on the GPU (still `UnsupportedText`).
- A general GPU LUT cache across composites; the table itself is already cached
  per gamma, and the per-composite upload is 16 KB.

## Decisions

**1. Upload the CPU table and interpolate it in the shader.**

The shader needs the CPU's *table*, not the exact formula. At γ 1.00 the table
deliberately differs from the exact curve by up to ~0.005 (the spec `tg_table`
bounds it at 0.01) near full coverage; an exact `pow` would match the curve but
could exceed the ±1 LSB parity contract. A 4097-entry storage buffer read with
the same samples and interpolation stays within that contract.

Alternatives rejected:

- A γ float plus `pow` in WGSL: fewer bindings, but diverges from the CPU table
  in the steep γ 1.00 region.
- Baking the curve into the source alpha on upload: the curve input includes
  per-dispatch opacity and Fill, so the cached source plane would have to be
  re-uploaded whenever either changed.

**2. A read-only storage binding (binding 4), not a uniform array.**

A uniform `array<f32>` has a 16-byte element stride, so 4097 entries would cost
about 64 KB and exceed the 64 KiB default uniform binding limit. A storage
buffer is 16 KB and is read like the existing source/mask planes. When the gamma
is off, a 4-byte dummy is bound and the shader's `text_on` flag keeps every read
behind the flag.

**3. A per-dispatch `text_on` flag, computed at the call site.**

`dispatch` takes a `text: bool`; the pixel, adjustment, and group branches pass
`layer.type_tool.is_some()`, which is false for a group because a group cannot
carry a type tool. The params uniform gains one `u32` at offset 68; the existing
80-byte buffer already has room. `Gpu` captures the current table and an
`text_on` availability flag once when it is created, so the flag and the bound
table cannot drift within one composite.

**4. Keep `GpuError::UnsupportedText`; rename only the metric.**

`ContentRoute::TypeSource` keeps returning `UnsupportedText`, so existing
matches and the public enum are unchanged. The decline reason
`unsupported_text` becomes `type_source`; a separate `text_gamma` bucket would
have no producer once the gate is gone, so it is not kept as a dead counter.
The GPU test asserts the counter does not move for a proxied type layer.

**5. Raise the device probe's storage-buffer minimum from 3 to 4.**

The blend pipeline now binds canvas, source, mask, and the curve table. Every
real Vulkan device supports at least 8 storage buffers per stage, but the probe
should name the actual requirement.

## Risks / Trade-offs

- [A device with exactly 3 storage buffers would fail pipeline creation] → the
  probe now rejects it as `Unavailable`; the CPU path remains.
- [Gamma changes between composites] → each `Gpu` captures the table at
  construction; a settings change already triggers a recomposite, which
  constructs a new `Gpu`.
- [16 KB upload per composite] → the table build is cached per gamma and the
  upload is one small `write_buffer`; cache it in `Devices` only if profiling
  shows it.
- [Text with effects still falls back] → unchanged and documented; #969's
  layer-source cache is the prerequisite for effects.

## Migration Plan

The control-server metrics reason key changes from `unsupported_text` to
`type_source`; update any consumer that reads the old key. Rollback restores the
old key along with the shader/binding and the eligibility check.
