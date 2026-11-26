## Context

The three Image Auto operations have selection-aware filtering and history; native adjustment kernels exist, but the direct renderer currently selects its native branch only for Apply Image. Levels and Curves adjustment layers already expose editable blocks and a preview/commit path. Adobe describes the intended control behavior while its exact correction kernels are unpublished. See [Adobe's CS6 adjustment guidance](https://helpx.adobe.com/ie/photoshop/using/making-quick-tonal-adjustments.html).

## Goals and non-goals

- Share one options dialog and one correction fit across the new Properties and configured Image Auto paths.
- Persist defaults only when an accepted dialog has Save as Defaults checked.
- Keep adjustment metadata in the existing PSD-compatible block formats and preserve direct commands' native depth.
- Leave additional Curves controls and the pre-existing destructive adjustment dialogs' Auto gestures to their own capabilities.

## Decisions

- Start the dialog and Properties Auto from saved profile values. Profiles without a saved override use Per Channel, 0.1% tails, black, neutral gray, white, and midtone snapping off. Factory Image commands keep their existing independent defaults and output.
- The dialog owns a preview transaction: flush any preceding Properties edit before opening, suspend its ordinary commit timer and gesture commits while previewing, and clear pending preview and commit timers on close. Accept applies the latest fit and commits one adjustment edit; cancel restores the opening block without a history state or preference write. A normal Auto click applies the saved settings once.
- Fit from the composite below the chosen adjustment, excluding the adjustment itself and layers above it. Stream source-resolution composite tiles into histogram and band statistics without allocating a full-size fitting image; do not average or downsample samples before computing percentiles. Use per-channel levels or sampled curve points on Properties; preserve alpha. PSD Levels and Curves records quantize endpoints to bytes, so a native range that collapses under that quantization remains identity.
- Route configured direct adjustments explicitly to authoritative native samples for eligible image layers; the current renderer chooses its native branch only for Apply Image, so the existing regional dispatch alone does not provide this. Match each command to its fixed algorithm, using saved midtone snapping only for Auto Color. Fit and apply native samples at native precision; preserve non-finite and out-of-unit-range HDR samples unchanged. Derive the fit from the selection rather than just blending through it. Preserve existing normal direct, Quick Mask, and layer-mask dispatch and selection blending; exercise those routes when adding native dispatch.
- Monochromatic contrast shares the source endpoints across channels. Per-channel contrast fits each independently. Find Dark & Light Colors estimates channel endpoints from the dark and light luminance bands. Snap uses near-neutral midtone statistics and a channel gamma; applying the same math in all supported depths keeps the configured paths consistent. Degenerate ranges remain identity.

## Risks and trade-offs

- The Properties fit streams large composites and Curves has finite control points. Numerical equivalence to Adobe's private kernel remains a manual comparison.
- Imported profiles can omit fields. The profile schema supplies defaults and sanitizes values before use. Preference failures are surfaced through the normal profile save error path.
