# Filter panes composite the layer as the commit will

## Why

The Shear pane and the Blur Gallery canvas draw a filtered copy of the active layer source-over a
base with that layer hidden. Only alpha, the raw raster mask, opacity and fill reach the copy, so
these previews ignore the layer's blend mode, Blend If, vector mask, mask density and feather, and
the selection, and disagree with the commit (#801). The reduced copy behind the other previews
keeps vector masks at document scale, so it misplaces them.

## What changes

- The Shear pane and the Blur Gallery canvas render the document through the reduced copy, with the
  filter on the active layer, replacing the base-plus-layer pair.
- The reduced copy scales vector-mask paths and feather.
- Catalog thumbnails are unchanged.

## Capabilities

- Modified: `imaging/filter-app-ui`, `imaging/filter-gallery-preview`, `imaging/blur-gallery`
