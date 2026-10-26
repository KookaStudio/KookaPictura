#include "encode_image.h"

#include "pictura_app/src/decode_bridge.cxxqt.h"

#include <QtCore/QBuffer>
#include <QtCore/QByteArray>
#include <QtCore/QSaveFile>
#include <QtCore/QString>
#include <QtCore/QtGlobal>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtGui/QImageWriter>
#include <QtGui/QPainter>

#include <algorithm>

namespace {

QString toQString(::rust::Str text)
{
    return QString::fromUtf8(text.data(), static_cast<int>(text.size()));
}

// `source` rescaled by `scale` percent with a smooth filter, or `source` itself
// at 100 (or an out-of-range) percent.
QImage scaledImage(const QImage& source, int32_t width, int32_t height, int32_t scale)
{
    if (scale <= 0 || scale == 100) {
        return source;
    }
    const int scaledWidth = std::max(1, qRound(width * scale / 100.0));
    const int scaledHeight = std::max(1, qRound(height * scale / 100.0));
    return source.scaled(scaledWidth, scaledHeight, Qt::KeepAspectRatio, Qt::SmoothTransformation);
}

// `image` composited onto an opaque `background` (a packed 0xRRGGBB).
QImage flattenedOnto(const QImage& image, int32_t background)
{
    const int r = (background >> 16) & 0xFF;
    const int g = (background >> 8) & 0xFF;
    const int b = background & 0xFF;
    QImage flat(image.width(), image.height(), QImage::Format_RGB32);
    flat.fill(QColor(r, g, b));
    QPainter painter(&flat);
    painter.drawImage(0, 0, image);
    painter.end();
    return flat;
}

// The encoded bytes of `rgba` in `format`, or empty on any refusal. `matte` is
// a packed 0xRRGGBB background, or -1 to keep alpha; JPEG and BMP always flatten
// (onto `matte`, else opaque white).
QByteArray encodeImage(::rust::Slice<const uint8_t> rgba, int32_t width, int32_t height,
                       ::rust::Str format, int32_t quality, int32_t scale, int32_t compression,
                       bool optimized, bool progressive, int32_t matte)
{
    if (width <= 0 || height <= 0) {
        return {};
    }
    if (rgba.size() < static_cast<size_t>(width) * static_cast<size_t>(height) * 4u) {
        return {};
    }

    const QImage source(rgba.data(), width, height, width * 4, QImage::Format_RGBA8888);
    QImage image = scaledImage(source, width, height, scale);

    const QString fmt = toQString(format).toUpper();
    const bool formatHasAlpha =
        !(fmt == QLatin1String("JPG") || fmt == QLatin1String("JPEG")
          || fmt == QLatin1String("BMP"));
    if (!formatHasAlpha) {
        // JPEG has no alpha, and Qt's BMP writer drops it too: composite onto
        // the matte (or opaque white) so the result is deterministic.
        image = flattenedOnto(image, matte >= 0 ? matte : 0xFFFFFF);
    } else if (matte >= 0) {
        // TIFF/PNG with Save Transparency off.
        image = flattenedOnto(image, matte);
    }

    const QByteArray formatBytes = fmt.toUtf8();
    const int qualityOrDefault = (quality > 0 && quality <= 100) ? quality : -1;
    QByteArray bytes;
    QBuffer buffer(&bytes);
    if (!buffer.open(QIODevice::WriteOnly)) {
        return {};
    }
    QImageWriter writer(&buffer, formatBytes);
    writer.setQuality(qualityOrDefault);
    if (compression >= 0) {
        writer.setCompression(compression);
    }
    writer.setOptimizedWrite(optimized);
    writer.setProgressiveScanWrite(progressive);
    if (!writer.write(image)) {
        return {};
    }
    return bytes;
}

} // namespace

bool encode_image_rgba(::rust::Slice<const uint8_t> rgba, int32_t width, int32_t height,
                       ::rust::Str path, ::rust::Str format, int32_t quality, int32_t scale,
                       int32_t compression, bool optimized, bool progressive, int32_t matte)
{
    const QByteArray bytes =
        encodeImage(rgba, width, height, format, quality, scale, compression, optimized,
                    progressive, matte);
    if (bytes.isEmpty()) {
        return false;
    }
    // QSaveFile writes to a temp file and renames on commit, so a mid-write
    // failure never truncates an existing file -- the same atomicity the PSD
    // codec path gets from tmp+rename.
    QSaveFile file(toQString(path));
    if (!file.open(QIODevice::WriteOnly)) {
        return false;
    }
    if (file.write(bytes) != bytes.size()) {
        file.cancelWriting();
        return false;
    }
    return file.commit();
}

::rust::Vec<uint8_t> encode_image_bytes(::rust::Slice<const uint8_t> rgba, int32_t width,
                                        int32_t height, ::rust::Str format, int32_t quality,
                                        int32_t scale, int32_t compression, bool optimized,
                                        bool progressive, int32_t matte)
{
    const QByteArray bytes =
        encodeImage(rgba, width, height, format, quality, scale, compression, optimized,
                    progressive, matte);
    if (bytes.isEmpty()) {
        return {};
    }
    return ::copy_bytes(::rust::Slice<const uint8_t>(
        reinterpret_cast<const uint8_t*>(bytes.constData()), static_cast<size_t>(bytes.size())));
}

::rust::Vec<uint8_t> scale_rgba(::rust::Slice<const uint8_t> rgba, int32_t width, int32_t height,
                                int32_t scale, int32_t& out_width, int32_t& out_height)
{
    out_width = 0;
    out_height = 0;
    if (width <= 0 || height <= 0) {
        return {};
    }
    if (rgba.size() < static_cast<size_t>(width) * static_cast<size_t>(height) * 4u) {
        return {};
    }
    const QImage source(rgba.data(), width, height, width * 4, QImage::Format_RGBA8888);
    const QImage out = scaledImage(source, width, height, scale)
                           .convertToFormat(QImage::Format_RGBA8888);
    if (out.isNull()) {
        return {};
    }
    out_width = out.width();
    out_height = out.height();
    const size_t bytes = static_cast<size_t>(out.width()) * static_cast<size_t>(out.height()) * 4u;
    return ::copy_bytes(::rust::Slice<const uint8_t>(out.constBits(), bytes));
}

::rust::Vec<uint8_t> encode_jpeg(::rust::Slice<const uint8_t> samples, int32_t width,
                                 int32_t height, int32_t components, int32_t quality)
{
    QImage::Format format = QImage::Format_Invalid;
    switch (components) {
    case 1:
        format = QImage::Format_Grayscale8;
        break;
    case 3:
        format = QImage::Format_RGB888;
        break;
    case 4:
        format = QImage::Format_CMYK8888;
        break;
    default:
        return {};
    }
    if (width <= 0 || height <= 0
        || samples.size()
               < static_cast<size_t>(width) * static_cast<size_t>(height)
                     * static_cast<size_t>(components)) {
        return {};
    }
    const QImage image(samples.data(), width, height, width * components, format);
    QByteArray bytes;
    QBuffer buffer(&bytes);
    buffer.open(QIODevice::WriteOnly);
    QImageWriter writer(&buffer, QByteArrayLiteral("JPEG"));
    writer.setQuality(std::clamp(quality, 0, 100));
    if (!writer.write(image)) {
        return {};
    }
    return ::copy_bytes(::rust::Slice<const uint8_t>(
        reinterpret_cast<const uint8_t*>(bytes.constData()), static_cast<size_t>(bytes.size())));
}
