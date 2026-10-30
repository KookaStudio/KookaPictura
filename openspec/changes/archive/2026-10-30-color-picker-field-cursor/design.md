# Design

The field pointer is a cursor asset rather than a painted overlay, so it works
in the dialog and in the Color panel's inline field, which both use
`ColorPlane`. `cursor.colorPickerField` follows the other cursor assets: a
24×24 viewBox, a white halo, and a dark line, centred on the (12, 12) hotspot.
`ColorPlane` sets it in its constructor and keeps `Qt::CrossCursor` as the
fallback when the resource is absent.

The ring matches `drawPlaneMarker`, the marker the field paints at the current
colour, so the pointer and the marker read as the same affordance.
