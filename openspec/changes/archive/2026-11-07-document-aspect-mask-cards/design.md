# Design

## Context

Mask images already sample the document, but maskThumbRect and
vectorMaskThumbRect reuse thumbnailCard, which forces adjustment, fill and type
cards square and preserves a group slot. The raster mask painter consequently
has a second fitting path. Plain row clicks can retain ActiveThumbRole from an
earlier mask selection. Adjustment creation already selects its new mask.

## Decisions

Use the existing document-aspect geometry helper for both mask card types.
Keep group-slot preservation in the content-card helper. Draw mask previews
directly in their document cards, and keep outlines, brackets and hit targets
using those same rectangles.

Handle the default content target only after dedicated row controls and explicit
thumbnail handlers have consumed their clicks. Clear raster editing/solo and
set the row's active thumbnail to content for an unmodified row-body click.
Ignore synthetic rows. Creation remains a programmatic operation, so its mask
activation is preserved.

Qt checks cover portrait/landscape documents, owner kinds, both mask types,
actual row clicks after raster/vector selection, explicit mask activation, and
adjustment creation. Write all checks before implementation and compile last.

## Risks / Trade-offs

Folder icons and adjustment/fill/type content cards keep their existing geometry.
The mask rectangles can differ from them; click tests exercise their true hit
targets. Existing square-mask expectations are superseded by the user's explicit
universal document-aspect requirement. No dependency or format changes.

The two legacy layout requirements are replaced under names that describe the
new policy. Their scenario tags and unrelated behaviors survive; the obsolete
square-mask scenario titles become document-aspect titles. This uses explicit
REMOVED/ADDED deltas rather than hiding a scenario rename in a MODIFIED block.

Fill fixtures keep their content kind: adding a vector mask would turn them into
shapes with a combined content/vector thumbnail. Their raster cards are tested;
separate vector cards are exercised on pixel, group, adjustment and type rows.
