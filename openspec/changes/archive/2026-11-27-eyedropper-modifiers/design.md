# Design

## Context

The Eyedropper handler samples through the scope-aware bridge, while Brush, Pencil, Color Replacement and the stamp/history family use separate point composite samplers. The sampler marker bridge already enforces the four-point limit and records annotation history. ToolController emits foreground and background colour signals to the shared colour state.

## Goals / Non-Goals

**Goals:** Reuse sampling options and annotation operations, keep the selected tool and its options stable, and preserve existing Alt gestures.

**Non-Goals:** Screen-wide sampling and new sampler persistence or readout features. Clone Stamp and Healing Brush use Alt for their source; Eraser uses Alt for history and Mixer Brush loads its reservoir. Those gestures remain intact. Other retouch tools are not included in this change.

## Decisions

- Route eligible temporary sampling gestures through the Eyedropper's existing scoped sampling logic, retaining the press-selected target for the gesture. This avoids divergent point-only sampling. An Alt gesture begun during a paint stroke must not steal that stroke; cancellation and tool changes clear the temporary gesture. Once temporary sampling starts, it and its cursor remain active until mouse release even if Alt is released first; the cursor then reflects the current modifiers.
- Resolve Shift on Eyedropper press as sampler placement before Alt colour selection; a placement is one action and does not turn a subsequent drag into colour sampling. Reuse `add_marker` and refusal handling rather than duplicating the document model.
- Route background sampling through the existing background colour signal path so all colour controls remain synchronized. Foreground remains unchanged.
- Allow eligible temporary sampling through the Quick Mask press gate without allowing a paint stroke. Use the same document sampling surface as the Eyedropper and leave Quick Mask coverage unchanged.

## Risks / Trade-offs

- [Alt ownership varies between tools] → Restrict the override to the six named tools and test the established conflicting gestures. The long-form document's broad wording is narrower here by explicit direction.
- [Modifier changes during drag] → Lock gesture choice and colour target at press to avoid accidental paint or changing both colours.
- [Sampling scope and size] → Exercise temporary sampling with non-default settings, and reuse the existing bridge and transparency behavior.
