#include "save_format_options.h"

#include <QtCore/QJsonValue>
#include <QtCore/QtGlobal>

namespace pictura {

namespace {

int clamped(int value, int low, int high)
{
    return value < low ? low : (value > high ? high : value);
}

} // namespace

SaveFormatKind saveFormatKindForSuffix(const QString& suffix)
{
    const QString lower = suffix.toLower();
    if (lower == QLatin1String("jpg") || lower == QLatin1String("jpeg")
        || lower == QLatin1String("jpe")) {
        return SaveFormatKind::Jpeg;
    }
    if (lower == QLatin1String("png")) {
        return SaveFormatKind::Png;
    }
    if (lower == QLatin1String("bmp")) {
        return SaveFormatKind::Bmp;
    }
    if (lower == QLatin1String("tif") || lower == QLatin1String("tiff")) {
        return SaveFormatKind::Tiff;
    }
    return SaveFormatKind::None;
}

QString saveFormatKey(SaveFormatKind kind)
{
    switch (kind) {
    case SaveFormatKind::Jpeg:
        return QStringLiteral("jpeg");
    case SaveFormatKind::Png:
        return QStringLiteral("png");
    case SaveFormatKind::Bmp:
        return QStringLiteral("bmp");
    case SaveFormatKind::Tiff:
        return QStringLiteral("tiff");
    default:
        return QString();
    }
}

QJsonObject saveFormatOptionsToJson(const SaveFormatOptions& options)
{
    QJsonObject jpeg;
    jpeg.insert(QStringLiteral("quality"), options.jpegQuality);
    jpeg.insert(QStringLiteral("format"), options.jpegFormat);
    jpeg.insert(QStringLiteral("scans"), options.jpegScans);
    jpeg.insert(QStringLiteral("matte"), options.jpegMatte);
    jpeg.insert(QStringLiteral("preview"), options.jpegPreview);

    QJsonObject png;
    png.insert(QStringLiteral("compression"), options.pngCompression);
    png.insert(QStringLiteral("interlace"), options.pngInterlace);

    QJsonObject bmp;
    bmp.insert(QStringLiteral("format"), options.bmpFormat);
    bmp.insert(QStringLiteral("depth"), options.bmpDepth);
    bmp.insert(QStringLiteral("rle"), options.bmpRle);
    bmp.insert(QStringLiteral("flip"), options.bmpFlip);

    QJsonObject tiff;
    tiff.insert(QStringLiteral("compression"), options.tiffCompression);
    tiff.insert(QStringLiteral("jpegQuality"), options.tiffJpegQuality);
    tiff.insert(QStringLiteral("pixelOrder"), options.tiffPixelOrder);
    tiff.insert(QStringLiteral("byteOrder"), options.tiffByteOrder);
    tiff.insert(QStringLiteral("layerCompression"), options.tiffLayerCompression);
    tiff.insert(QStringLiteral("pyramid"), options.tiffPyramid);
    tiff.insert(QStringLiteral("transparency"), options.tiffTransparency);

    QJsonObject root;
    root.insert(QStringLiteral("jpeg"), jpeg);
    root.insert(QStringLiteral("png"), png);
    root.insert(QStringLiteral("bmp"), bmp);
    root.insert(QStringLiteral("tiff"), tiff);
    return root;
}

SaveFormatOptions saveFormatOptionsFromJson(const QJsonObject& root)
{
    SaveFormatOptions options;
    const QJsonObject jpeg = root.value(QStringLiteral("jpeg")).toObject();
    options.jpegQuality = clamped(jpeg.value(QStringLiteral("quality")).toInt(options.jpegQuality), 0, 12);
    options.jpegFormat = clamped(jpeg.value(QStringLiteral("format")).toInt(options.jpegFormat), 0, 2);
    options.jpegScans = clamped(jpeg.value(QStringLiteral("scans")).toInt(options.jpegScans), 1, 100);
    options.jpegMatte = jpeg.value(QStringLiteral("matte")).toInt(options.jpegMatte);
    options.jpegPreview = jpeg.value(QStringLiteral("preview")).toBool(options.jpegPreview);

    const QJsonObject png = root.value(QStringLiteral("png")).toObject();
    options.pngCompression = clamped(png.value(QStringLiteral("compression")).toInt(options.pngCompression), 0, 100);
    options.pngInterlace = clamped(png.value(QStringLiteral("interlace")).toInt(options.pngInterlace), 0, 1);

    const QJsonObject bmp = root.value(QStringLiteral("bmp")).toObject();
    options.bmpFormat = clamped(bmp.value(QStringLiteral("format")).toInt(options.bmpFormat), 0, 1);
    options.bmpDepth = clamped(bmp.value(QStringLiteral("depth")).toInt(options.bmpDepth), 1, 32);
    options.bmpRle = bmp.value(QStringLiteral("rle")).toBool(options.bmpRle);
    options.bmpFlip = bmp.value(QStringLiteral("flip")).toBool(options.bmpFlip);

    const QJsonObject tiff = root.value(QStringLiteral("tiff")).toObject();
    options.tiffCompression = tiff.value(QStringLiteral("compression")).toInt(options.tiffCompression);
    options.tiffJpegQuality = clamped(tiff.value(QStringLiteral("jpegQuality")).toInt(options.tiffJpegQuality), 0, 12);
    options.tiffPixelOrder = clamped(tiff.value(QStringLiteral("pixelOrder")).toInt(options.tiffPixelOrder), 0, 1);
    options.tiffByteOrder = clamped(tiff.value(QStringLiteral("byteOrder")).toInt(options.tiffByteOrder), 0, 1);
    options.tiffLayerCompression = clamped(tiff.value(QStringLiteral("layerCompression")).toInt(options.tiffLayerCompression), 0, 2);
    options.tiffPyramid = tiff.value(QStringLiteral("pyramid")).toBool(options.tiffPyramid);
    options.tiffTransparency = tiff.value(QStringLiteral("transparency")).toBool(options.tiffTransparency);
    return options;
}

int jpegQualityPercent(int quality)
{
    // Photoshop's Save As 0-12 is a high-quality range, but its lowest setting
    // is still well above the libjpeg floor: on a reference photo it produces a
    // file close to libjpeg quality 60. The shape follows the reverse-engineered
    // Photoshop-to-libjpeg mapping (Raphaël Quinet, GIMP 2.4, 2007), lowered and
    // stretched at the low end so quality 0 matches the file size a CS6 Save As
    // at 0 produces. Adobe's exact quantisation tables are unpublished, so this
    // is an approximation. ponytail: it ignores Photoshop's chroma-subsampling
    // switch, which QImageWriter cannot set.
    static const int kPercent[13] = {60, 64, 68, 72, 75, 78, 81, 84, 86, 89, 92, 95, 98};
    return kPercent[clamped(quality, 0, 12)];
}

} // namespace pictura
