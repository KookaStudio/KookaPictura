#pragma once

#include <QtGui/QColor>
#include <QtGui/QIcon>

namespace pictura::character_icons {

// The Character panel's icons, drawn at runtime from type and strokes in
// `color` (no Adobe artwork). Field icons label the numeric fields; style and
// OpenType icons face the toggle buttons, in panel order.
enum class Field {
    Size,
    Leading,
    Kerning,
    Tracking,
    VerticalScale,
    HorizontalScale,
    BaselineShift,
    AntiAlias,
};

QIcon field(Field which, const QColor& color);

// 0 Faux Bold, 1 Faux Italic, 2 All Caps, 3 Small Caps, 4 Superscript,
// 5 Subscript, 6 Underline, 7 Strikethrough.
QIcon style(int index, const QColor& color);

// 0 Standard Ligatures, 1 Contextual Alternates, 2 Discretionary Ligatures,
// 3 Swash, 4 Stylistic Alternates, 5 Titling Alternates, 6 Ordinals,
// 7 Fractions.
QIcon openType(int index, const QColor& color);

} // namespace pictura::character_icons
