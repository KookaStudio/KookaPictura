#pragma once

#include <QColor>
#include <QString>

namespace pictura {

/// Snap each channel to the nearest web-safe value (multiples of 0x33).
QColor snapToWebColor(const QColor& color);
/// Whether every channel is already a multiple of 0x33.
bool isWebColor(const QColor& color);

// Shared by the picker widgets; not part of the public dialog API.
void toHsvPreservingHue(const QColor& color, int& h, int& s, int& v);
/// Strip an optional `#`/`0x` prefix and surrounding whitespace.
QString hexDigits(QString text);
bool parseHexColor(const QString& text, QColor* color);

} // namespace pictura
