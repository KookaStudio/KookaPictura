#include "color_models.h"

#include "pictura_app/src/cxxqt_object/color_models.cxxqt.h"

#include <cstdint>
#include <vector>

namespace pictura {

void rgbToLab(const QColor& color, double* l, double* a, double* b)
{
    const QColor rgb = color.toRgb();
    const LabValue lab = srgb_to_lab(std::uint8_t(rgb.red()), std::uint8_t(rgb.green()),
                                     std::uint8_t(rgb.blue()));
    *l = lab.l;
    *a = lab.a;
    *b = lab.b;
}

QColor labToRgb(double l, double a, double b)
{
    const QRgb rgb = QRgb(lab_to_srgb(l, a, b));
    return QColor(qRed(rgb), qGreen(rgb), qBlue(rgb));
}

CmykPercent rgbToCmyk(const QColor& color)
{
    const QColor rgb = color.toRgb();
    const CmykValue inks = srgb_to_cmyk(std::uint8_t(rgb.red()), std::uint8_t(rgb.green()),
                                        std::uint8_t(rgb.blue()));
    return {inks.c, inks.m, inks.y, inks.k};
}

QVector<uchar> channelPlane(const QImage& image, const QString& mode, int row)
{
    const QImage argb = image.convertToFormat(QImage::Format_ARGB32);
    std::vector<std::uint32_t> pixels;
    pixels.reserve(std::size_t(argb.width()) * std::size_t(argb.height()));
    for (int y = 0; y < argb.height(); ++y) {
        const auto* line = reinterpret_cast<const QRgb*>(argb.constScanLine(y));
        pixels.insert(pixels.end(), line, line + argb.width());
    }
    const ::rust::Vec<std::uint8_t> plane =
        channel_plane(::rust::Slice<const std::uint32_t>(pixels.data(), pixels.size()), mode, row);
    return QVector<uchar>(plane.begin(), plane.end());
}

} // namespace pictura
