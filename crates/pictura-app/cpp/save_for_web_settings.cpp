#include "save_for_web_settings.h"

#include <QtCore/QBuffer>
#include <QtGui/QPainter>
#include <QtGui/QImageWriter>

#include <cstddef>
#include <cstdint>
#include <cstring>

namespace pictura {

namespace {

::rust::Slice<const std::uint8_t> bytesOf(const QByteArray& data)
{
    return {reinterpret_cast<const std::uint8_t*>(data.constData()),
            std::size_t(data.size())};
}

::rust::Slice<const std::uint8_t> bytesOf(const QImage& image)
{
    return {image.constBits(), std::size_t(image.sizeInBytes())};
}

QByteArray fromRust(const ::rust::Vec<std::uint8_t>& data)
{
    return QByteArray(reinterpret_cast<const char*>(data.data()), qsizetype(data.size()));
}

QImage over(const QImage& image, const QColor& matte)
{
    QImage flat(image.size(), QImage::Format_RGB32);
    flat.fill(matte);
    QPainter painter(&flat);
    painter.drawImage(0, 0, image);
    return flat;
}

QColor matteColor(const WebSettings& settings)
{
    return settings.matte == 1 ? QColor(Qt::black) : QColor(Qt::white);
}

} // namespace

WebPalette paletteOf(const WebSettings& settings)
{
    WebPalette palette{};
    palette.reduction = settings.reduction;
    palette.colors = settings.colors;
    palette.dither = settings.dither;
    palette.amount = settings.ditherAmount;
    palette.transparency = settings.transparency;
    palette.mapped_transparency = settings.mappedTransparency;
    palette.matte = settings.matte == 1 ? 0xff000000u : 0xffffffffu;
    return palette;
}

QByteArray quantizeWebImage(const QImage& image, const WebSettings& settings)
{
    if (settings.packed.size() >= 2) {
        const int n = (int(quint8(settings.packed[0])) << 8)
            | int(quint8(settings.packed[1]));
        if (n > 0
            && settings.packed.size()
                == 2 + n * 4 + image.width() * image.height()) {
            return settings.packed;
        }
    }
    const QImage rgba = image.convertToFormat(QImage::Format_RGBA8888);
    const WebPalette palette = paletteOf(settings);
    ::rust::Vec<std::uint8_t> packed;
    if (!settings.custom.isEmpty()) {
        QByteArray table;
        table.reserve(settings.custom.size() * 4);
        for (QRgb color : settings.custom) {
            table.append(char(qRed(color)));
            table.append(char(qGreen(color)));
            table.append(char(qBlue(color)));
            table.append(char(qAlpha(color)));
        }
        packed = web_quantize_custom(bytesOf(rgba), rgba.width(), rgba.height(), palette,
                                     bytesOf(table));
    } else if (!settings.locked.isEmpty()) {
        QByteArray locked;
        locked.reserve(settings.locked.size() * 3);
        for (QRgb color : settings.locked) {
            locked.append(char(qRed(color)));
            locked.append(char(qGreen(color)));
            locked.append(char(qBlue(color)));
        }
        packed = web_quantize_locked(bytesOf(rgba), rgba.width(), rgba.height(), palette,
                                     bytesOf(locked));
    } else {
        packed = web_quantize(bytesOf(rgba), rgba.width(), rgba.height(), palette);
    }
    return fromRust(packed);
}

QImage indexedImageFromPacked(const QByteArray& packed, const QSize& size)
{
    if (packed.size() < 2) {
        return QImage();
    }
    const int n = (int(quint8(packed[0])) << 8) | int(quint8(packed[1]));
    if (n < 1 || packed.size() != 2 + n * 4 + size.width() * size.height()) {
        return QImage();
    }
    QImage indexed(size, QImage::Format_Indexed8);
    QList<QRgb> table;
    for (int i = 0; i < n; ++i) {
        const auto* c = reinterpret_cast<const std::uint8_t*>(packed.constData()) + 2 + i * 4;
        table.append(qRgba(c[0], c[1], c[2], c[3]));
    }
    indexed.setColorTable(table);
    const char* indices = packed.constData() + 2 + n * 4;
    for (int y = 0; y < size.height(); ++y) {
        std::memcpy(indexed.scanLine(y), indices + qsizetype(y) * size.width(),
                    std::size_t(size.width()));
    }
    return indexed;
}

QVector<QRgb> paletteFromPacked(const QByteArray& packed)
{
    QVector<QRgb> colors;
    if (packed.size() < 2) {
        return colors;
    }
    const int n = (int(quint8(packed[0])) << 8) | int(quint8(packed[1]));
    if (n < 1 || packed.size() < 2 + n * 4) {
        return colors;
    }
    for (int i = 0; i < n; ++i) {
        const auto* c = reinterpret_cast<const std::uint8_t*>(packed.constData()) + 2 + i * 4;
        colors.append(qRgba(c[0], c[1], c[2], c[3]));
    }
    return colors;
}

QString webSuffix(const WebSettings& settings)
{
    switch (settings.format) {
    case 1:
        return QStringLiteral("jpg");
    case 2:
    case 3:
        return QStringLiteral("png");
    case 4:
        return QStringLiteral("wbmp");
    default:
        return QStringLiteral("gif");
    }
}

QByteArray encodeWebImage(const QImage& image, const WebSettings& settings)
{
    QByteArray out;
    QBuffer buffer(&out);
    buffer.open(QIODevice::WriteOnly);
    const QColor matte = matteColor(settings);
    switch (settings.format) {
    case 1: {
        QImageWriter writer(&buffer, "jpeg");
        writer.setQuality(settings.quality);
        writer.setOptimizedWrite(settings.optimizedJpeg);
        writer.setProgressiveScanWrite(settings.progressive);
        writer.write(over(image, matte));
        break;
    }
    case 2: {
        const QByteArray packed = quantizeWebImage(image, settings);
        QImageWriter(&buffer, "png").write(
            indexedImageFromPacked(packed, image.size()));
        break;
    }
    case 3:
        QImageWriter(&buffer, "png")
            .write(settings.pngTransparency ? image : over(image, matte));
        break;
    case 4: {
        const QImage rgba = image.convertToFormat(QImage::Format_RGBA8888);
        const ::rust::Vec<std::uint8_t> bytes =
            web_encode_wbmp(bytesOf(rgba), rgba.width(), rgba.height(),
                            settings.wbmpDither, settings.wbmpAmount);
        out = fromRust(bytes);
        break;
    }
    default: {
        const QByteArray packed = quantizeWebImage(image, settings);
        if (packed.size() >= 2) {
            const ::rust::Vec<std::uint8_t> bytes =
                web_encode_gif(bytesOf(packed), image.width(), image.height(),
                               settings.interlaced);
            out = fromRust(bytes);
        }
        break;
    }
    }
    return out;
}

QByteArray embedWebMetadata(const QByteArray& data, const QString& suffix,
                            const WebMetadata& raw, int level)
{
    const ::rust::Vec<std::uint8_t> bytes =
        web_embed_metadata(bytesOf(data), suffix, raw, level);
    return fromRust(bytes);
}

WebSettings loweredSettings(const WebSettings& settings, int step)
{
    WebSettings out = settings;
    const bool palette = settings.format == 0 || settings.format == 2;
    if (palette) {
        out.colors = qMax(2, settings.colors >> step);
        out.custom.clear();
        out.locked.clear();
        out.packed.clear();
        out.mappedTransparency = false;
    } else if (settings.format == 1) {
        out.quality = qMax(1, settings.quality - 20 * step);
    }
    return out;
}

} // namespace pictura
