# Design

## One band for the label, paint, hit-testing, and the editor cap

`nameRect` already computes the label's left edge (after the thumbnail block,
the name gap, and the clipping indent) and the right cap (the row's right edge
minus the lock, list-toggle, fx, link, and Blend If badges). Its width is the
elided text's advance, which is what the double-click hit target needs.

The editor uses both rects. It starts at the rendered label's left edge and is
as wide as the rendered text plus the name gap, so a short name gets a small
field around the text instead of a field spanning the empty band. The width is
capped at `nameBandRect`, so a long name still stops before the trailing badges.

Extract the band computation into `nameBandRect(itemRect, index)` and have
`nameRect` elide inside it. The band is the reserved label space; the rendered
label is the part of it the name occupies.

## Editor geometry

`QStyledItemDelegate::updateEditorGeometry` defaults to `option.rect`, the whole
row. The delegate now overrides it:

1. Take `nameRect(option.rect, index, option.font)`.
2. Width = `min(nameBandRect(...).width(), name.width() + kNameGap)`.
3. Place the editor at the name rect's top, left, and height.

A short label therefore gets a field just past its text; a name that fills the
band gets the full band; every row stops before the badges that are present.
`createEditor` and `setEditorData` keep the base behavior, so the edit remains a
plain `QLineEdit` with select-all on open and the existing Tab/Shift+Tab
navigation.
