#include "encode_image.h"

#include "pictura_app/src/decode_bridge.cxxqt.h"

#include <QtCore/QByteArray>
#include <QtCore/QSaveFile>
#include <QtCore/QString>
#include <QtCore/QtGlobal>
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

} // namespace

bool encode_image_rgba(::rust::Slice<const uint8_t> rgba, int32_t width, int32_t height,
                       ::rust::Str path, ::rust::Str format, int32_t quality, int32_t scale)
{
    if (width <= 0 || height <= 0) {
        return false;
    }
    if (rgba.size() < static_cast<size_t>(width) * static_cast<size_t>(height) * 4u) {
        return false;
    }

    const QImage source(rgba.data(), width, height, width * 4, QImage::Format_RGBA8888);

    QImage image = scaledImage(source, width, height, scale);

    const QString fmt = toQString(format).toUpper();
    if (fmt == QLatin1String("JPG") || fmt == QLatin1String("JPEG")
        || fmt == QLatin1String("BMP")) {
        // JPEG has no alpha, and Qt's BMP writer drops it too: composite the
        // transparent pixels onto opaque white so the result is deterministic.
        QImage flat(image.width(), image.height(), QImage::Format_RGB32);
        flat.fill(Qt::white);
        QPainter painter(&flat);
        painter.drawImage(0, 0, image);
        painter.end();
        image = flat;
    }

    const QByteArray formatBytes = fmt.toUtf8();
    const int qualityOrDefault = (quality > 0 && quality <= 100) ? quality : -1;
    // QSaveFile writes to a temp file and renames on commit, so a mid-write
    // failure never truncates an existing file -- the same atomicity the PSD
    // codec path gets from tmp+rename.
    QSaveFile file(toQString(path));
    if (!file.open(QIODevice::WriteOnly)) {
        return false;
    }
    QImageWriter writer(&file, formatBytes);
    writer.setQuality(qualityOrDefault);
    if (!writer.write(image)) {
        file.cancelWriting();
        return false;
    }
    return file.commit();
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
