#pragma once

#include <QtCore/QString>
#include <QtCore/QVector>
#include <QtGui/QColor>
#include <QtGui/QImage>

namespace pictura {

// The colour-model conversions every readout shares. The math lives in the
// engine; these only adapt QColor/QImage to the bridge.

/// sRGB -> CIE L*a*b* (D50), the space Photoshop's Lab readout uses.
void rgbToLab(const QColor& color, double* l, double* a, double* b);
/// CIE L*a*b* (D50) -> sRGB, clamped into gamut.
QColor labToRgb(double l, double a, double b);

/// Whole CMYK ink percentages (0-100) through the installed working CMYK profile.
struct CmykPercent {
    double c = 0.0;
    double m = 0.0;
    double y = 0.0;
    double k = 0.0;
};
CmykPercent rgbToCmyk(const QColor& color);

/// Row `row`'s grey value per pixel of `image` (row-major) for a `mode`
/// document, as the Channels panel thumbnails and channel-to-selection share.
QVector<uchar> channelPlane(const QImage& image, const QString& mode, int row);

} // namespace pictura
