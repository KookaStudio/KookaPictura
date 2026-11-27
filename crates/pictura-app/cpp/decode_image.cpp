#include "decode_image.h"

#include "api/engine_bridge.h"

#include <QtCore/QBuffer>
#include <QtGui/QImage>
#include <QtGui/QImageReader>

namespace {

// pictura_codec::ImageBudget::default(): 30 000 px a side and 2 GiB of RGBA.
// The engine re-checks the decoded size; these keep Qt from allocating past it
// first (#844 H9).
constexpr qint64 kMaxDimension = 30000;
constexpr qint64 kMaxAllocBytes = qint64(2) * 1024 * 1024 * 1024;

bool withinBudget(qint64 width, qint64 height)
{
    return width > 0 && height > 0 && width <= kMaxDimension && height <= kMaxDimension
           && width * height * 4 <= kMaxAllocBytes;
}

// No image of any format Qt or its plugins decode fits in fewer bytes; shorter
// input never reaches a plugin, some of which read their magic bytes past the
// end of the device (kimg_exr on an empty file, #1650).
constexpr qsizetype kMinImageBytes = 16;

} // namespace

QByteArray pictura::qtFormatForSignature(const QByteArray& head)
{
    // The same signatures as pictura_codec::probe's sniffer.
    if (head.startsWith("\x89PNG\r\n\x1a\n")) {
        return QByteArrayLiteral("png");
    }
    if (head.startsWith("\xff\xd8")) {
        return QByteArrayLiteral("jpeg");
    }
    if (head.startsWith("GIF87a") || head.startsWith("GIF89a")) {
        return QByteArrayLiteral("gif");
    }
    if (head.startsWith("BM")) {
        return QByteArrayLiteral("bmp");
    }
    if (head.startsWith(QByteArrayView("II\x2a\x00", 4))
        || head.startsWith(QByteArrayView("MM\x00\x2a", 4))) {
        return QByteArrayLiteral("tiff");
    }
    return {};
}

::rust::Vec<uint8_t> decode_image_rgba(::rust::Slice<const uint8_t> data, int32_t& width,
                                       int32_t& height)
{
    width = 0;
    height = 0;
    if (data.size() < size_t(kMinImageBytes)) {
        return {};
    }
    // Qt's QImageReader refuses any allocation over its 256 MB default, which
    // would reject a 16000²-class raster (a 16507×16196 RGBA decode is ~1020 MB)
    // before the app's own budget sees it, so its limit is the import budget's
    // (in MB). Static and idempotent; read when the reader is constructed below.
    QImageReader::setAllocationLimit(static_cast<int>(kMaxAllocBytes / (1024 * 1024)));
    const QByteArray bytes = QByteArray::fromRawData(reinterpret_cast<const char*>(data.data()),
                                                     static_cast<qsizetype>(data.size()));
    QBuffer buffer;
    buffer.setData(bytes);
    if (!buffer.open(QIODevice::ReadOnly)) {
        return {};
    }
    QImageReader reader(&buffer);
    // A known signature consults only its own handler, never the content scan
    // over every installed plugin.
    const QByteArray pinned = pictura::qtFormatForSignature(bytes);
    if (!pinned.isEmpty()) {
        reader.setFormat(pinned);
        reader.setAutoDetectImageFormat(false);
    }
    // Refuse a declared size over budget before any plugin decodes pixels; a
    // container the engine's probe does not know reaches here unchecked.
    const QSize declared = reader.size();
    if (declared.isValid() && !withinBudget(declared.width(), declared.height())) {
        return {};
    }
    const QImage decoded = reader.read();
    // A truncated file can still yield a partial image (missing rows filled
    // transparent); reject the reader's error instead of installing it.
    if (decoded.isNull() || reader.error() != QImageReader::UnknownError
        || !withinBudget(decoded.width(), decoded.height())) {
        return {};
    }
    const QImage rgba = decoded.convertToFormat(QImage::Format_RGBA8888);
    if (rgba.isNull()) {
        return {};
    }
    width = rgba.width();
    height = rgba.height();
    const qsizetype stride = qsizetype(width) * 4;
    // A Format_RGBA8888 image is a single contiguous buffer, so hand the whole
    // frame to Rust in one copy instead of pushing a billion bytes one at a time.
    return ::copy_bytes(::rust::Slice<const uint8_t>(
        rgba.constBits(), static_cast<size_t>(stride) * static_cast<size_t>(height)));
}
