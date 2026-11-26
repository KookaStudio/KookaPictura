# Design

## Context

The Levels dialog edits existing composite and component records. Its adjustment preview can stage edits asynchronously. The dialog host already allows canvas clicks while a ToolController sampler is armed, and ImageView can show a temporary image over its composite.

## Goals

- Give the destructive Levels dialog in-image sampling and transient clipping feedback.
- Keep the sampled image independent of the adjustment being edited.
- Keep clipping feedback independent of the stored Levels parameters and document history.

## Decisions

### Sample the unadjusted canvas

Temporarily cancel the live adjustment before reading the image. Read and average only the chosen neighborhood of the unadjusted visible composite through a bounded picture-region read or existing equivalent; the scoped sampler recomposites the whole document and is not suitable for this interaction. A right-click sample-size menu offers Point (the default), 3×3, 5×5, 11×11, 31×31, 51×51, and 101×101. Retain the sample size and neutral targets within the currently open dialog.

Translate valid samples to an identity-based Levels block in one edit. For each affected component, a black pick sets input black to the sample and output black to the neutral target; a white pick sets input white to the sample and output white to the target. In RGB use the component records; in Grayscale use the composite record and disable gray-point picking. Reject the entire black pick if any affected sample is outside 0–253, and the entire white pick if any is outside 2–255, preserving the minimum input spread. For a gray pick, solve each component's gamma for the neutral integer target in 1–254, bounded by the existing gamma limits; reject a sample containing an endpoint. Ignore fully transparent samples. A rejected pick preserves the block and restores the user's regular Preview setting, including scheduling a pending or previously displayed preview when Preview is on.

The new interactions use the current RGB and Grayscale level records and are disabled in other modes and when the adjustment targets a mask. Existing numeric controls remain available. Read displayed 8-bit samples, in line with the current 0–255 Levels controls.

### Draw a display-only clipping map

Track Alt and the active input thumb. Use a pre-preview composite region from the display pyramid at a bounded resolution and classify each visible sample against the current threshold. A shadow map starts white and clears the clipping components. A highlight map starts black and lights the clipping components. A single selected channel gives a monochrome map. Preserve transparent coverage and use the canvas overlay, with no edits beyond the ordinary input-slider change. Resume regular preview upon mouse release, Alt release, or loss of the gesture. The clipping map works while regular Preview is unchecked.

When the configured pyramid is too shallow for a bounded crop, composite and reduce the visible region in chunks at the required level. Cache that source for the visible crop during the gesture so changing the threshold does not recomposite it.

## Risks and trade-offs

- The display pyramid can average details when zoomed out. Classifications use the displayed source resolution, and native-depth or CS6 pixel-identical clipping is outside the acceptance criteria.
- In a layered image the picked color comes from the visible composite, while destructive Levels applies to its normal pixel target.
- Colors already at an extreme and very high or low required gamma are bounded by the current Levels controls.
