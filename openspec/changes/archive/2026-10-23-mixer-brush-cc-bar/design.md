# Design: Mixer Brush bar and Sample All Layers

## Sample All Layers

`Stroke::begin_mixer` wraps `begin_kind(StrokeKind::Mixer)` and hands the engine a layer-local copy of the document composite, the same way `begin_smudge` and `begin_focus` do. `MixerBrush::dab` averages the pickup from that copy when present and from the layer otherwise; the deposit and the layer write are unchanged.

The composite is a snapshot at the press, so earlier dabs of the same stroke do not show in the pickup while the option is on. This is a deliberate shortcut (`ponytail:` in `mixer.rs`), shared with Smudge.

## Bar

Mode and Opacity leave `buildMixerBrushPage`. Because `PaintOptions` is shared with the Brush, the tool now passes Normal and 100 explicitly; otherwise a Brush at 30 % would silently thin Mixer strokes. `sampleAllLayers` joins `MixerOptions` and its session JSON.

## Approximation

No Photoshop CC reference exists for the mixing. The tests assert orderings and engine behaviour, not CC parity. Known limits: one flat load colour, a tip-weighted mean pickup instead of Adobe's bristle model, and photorust's preset values and rates.
