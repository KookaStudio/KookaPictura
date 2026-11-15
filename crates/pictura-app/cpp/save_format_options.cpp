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
    if (lower == QLatin1String("gif")) {
        return SaveFormatKind::Gif;
    }
    if (lower == QLatin1String("eps")) {
        return SaveFormatKind::Eps;
    }
    if (lower == QLatin1String("raw")) {
        return SaveFormatKind::Raw;
    }
    if (lower == QLatin1String("tga") || lower == QLatin1String("vda")
        || lower == QLatin1String("icb") || lower == QLatin1String("vst")) {
        return SaveFormatKind::Tga;
    }
    if (lower == QLatin1String("cin")) {
        return SaveFormatKind::Cineon;
    }
    return SaveFormatKind::None;
}

SaveFormatKind saveFormatKindForFilter(int filterIndex)
{
    switch (filterIndex) {
    case 2:
        return SaveFormatKind::Png;
    case 3:
        return SaveFormatKind::Jpeg;
    case 4:
        return SaveFormatKind::Tiff;
    case 6:
        return SaveFormatKind::Bmp;
    case 8:
        return SaveFormatKind::Gif;
    case 9:
        return SaveFormatKind::Eps;
    case 10:
        return SaveFormatKind::Dcs;
    case 11:
        return SaveFormatKind::Raw;
    case 12:
        return SaveFormatKind::Tga;
    case 13:
        return SaveFormatKind::Cineon;
    default:
        return SaveFormatKind::None;
    }
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
    case SaveFormatKind::Gif:
        return QStringLiteral("gif");
    case SaveFormatKind::Eps:
        return QStringLiteral("eps");
    case SaveFormatKind::Dcs:
        return QStringLiteral("dcs");
    case SaveFormatKind::Raw:
        return QStringLiteral("raw");
    case SaveFormatKind::Tga:
        return QStringLiteral("tga");
    case SaveFormatKind::Cineon:
        return QStringLiteral("cineon");
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
    bmp.insert(QStringLiteral("advanced"), options.bmpAdvanced);

    QJsonObject tiff;
    // Version 2 uses the engine codes (0 None, 1 RLE, 2 LZW, 3 ZIP, 4 JPEG).
    // Version 1 was Qt's enum (0 None, 1 LZW, 5 JPEG).
    tiff.insert(QStringLiteral("version"), 2);
    tiff.insert(QStringLiteral("compression"), options.tiffCompression);
    tiff.insert(QStringLiteral("jpegQuality"), options.tiffJpegQuality);
    tiff.insert(QStringLiteral("pixelOrder"), options.tiffPixelOrder);
    tiff.insert(QStringLiteral("byteOrder"), options.tiffByteOrder);
    tiff.insert(QStringLiteral("layerCompression"), options.tiffLayerCompression);
    tiff.insert(QStringLiteral("pyramid"), options.tiffPyramid);
    tiff.insert(QStringLiteral("transparency"), options.tiffTransparency);

    QJsonObject gif;
    gif.insert(QStringLiteral("rowOrder"), options.gifRowOrder);

    QJsonObject eps;
    eps.insert(QStringLiteral("preview"), options.epsPreview);
    eps.insert(QStringLiteral("encoding"), options.epsEncoding);
    eps.insert(QStringLiteral("jpegQuality"), options.epsJpegQuality);
    eps.insert(QStringLiteral("halftone"), options.epsHalftone);
    eps.insert(QStringLiteral("transfer"), options.epsTransfer);
    eps.insert(QStringLiteral("transparentWhites"), options.epsTransparentWhites);
    eps.insert(QStringLiteral("psColorManagement"), options.epsPsColorManagement);
    eps.insert(QStringLiteral("vectorData"), options.epsVectorData);
    eps.insert(QStringLiteral("interpolation"), options.epsInterpolation);

    QJsonObject dcs;
    dcs.insert(QStringLiteral("version"), options.dcsVersion);
    dcs.insert(QStringLiteral("preview"), options.dcsPreview);

    QJsonObject raw;
    raw.insert(QStringLiteral("header"), options.rawHeader);
    raw.insert(QStringLiteral("interleave"), options.rawInterleave);

    QJsonObject tga;
    tga.insert(QStringLiteral("depth"), options.tgaDepth);
    tga.insert(QStringLiteral("rle"), options.tgaRle);

    QJsonObject cineon;
    cineon.insert(QStringLiteral("bits"), options.cineonBits);

    QJsonObject root;
    root.insert(QStringLiteral("jpeg"), jpeg);
    root.insert(QStringLiteral("png"), png);
    root.insert(QStringLiteral("bmp"), bmp);
    root.insert(QStringLiteral("tiff"), tiff);
    root.insert(QStringLiteral("gif"), gif);
    root.insert(QStringLiteral("eps"), eps);
    root.insert(QStringLiteral("dcs"), dcs);
    root.insert(QStringLiteral("raw"), raw);
    root.insert(QStringLiteral("tga"), tga);
    root.insert(QStringLiteral("cineon"), cineon);
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
    options.bmpAdvanced = clamped(bmp.value(QStringLiteral("advanced")).toInt(options.bmpAdvanced), 0, 2);

    const QJsonObject tiff = root.value(QStringLiteral("tiff")).toObject();
    int tiffCompression = tiff.value(QStringLiteral("compression")).toInt(options.tiffCompression);
    if (tiff.value(QStringLiteral("version")).toInt(1) < 2) {
        // An older session stored Qt's compression ids. 1 was LZW and 5 JPEG;
        // map them before the engine ids took over. 0 was and is None.
        tiffCompression = tiffCompression == 1 ? 2 : (tiffCompression == 5 ? 4 : tiffCompression);
    }
    options.tiffCompression = clamped(tiffCompression, 0, 4);
    options.tiffJpegQuality = clamped(tiff.value(QStringLiteral("jpegQuality")).toInt(options.tiffJpegQuality), 0, 12);
    options.tiffPixelOrder = clamped(tiff.value(QStringLiteral("pixelOrder")).toInt(options.tiffPixelOrder), 0, 1);
    options.tiffByteOrder = clamped(tiff.value(QStringLiteral("byteOrder")).toInt(options.tiffByteOrder), 0, 1);
    options.tiffLayerCompression = clamped(tiff.value(QStringLiteral("layerCompression")).toInt(options.tiffLayerCompression), 0, 2);
    options.tiffPyramid = tiff.value(QStringLiteral("pyramid")).toBool(options.tiffPyramid);
    options.tiffTransparency = tiff.value(QStringLiteral("transparency")).toBool(options.tiffTransparency);

    const QJsonObject gif = root.value(QStringLiteral("gif")).toObject();
    options.gifRowOrder = clamped(gif.value(QStringLiteral("rowOrder")).toInt(options.gifRowOrder), 0, 1);

    const QJsonObject eps = root.value(QStringLiteral("eps")).toObject();
    options.epsPreview = clamped(eps.value(QStringLiteral("preview")).toInt(options.epsPreview), 0, 2);
    options.epsEncoding = clamped(eps.value(QStringLiteral("encoding")).toInt(options.epsEncoding), 0, 3);
    options.epsJpegQuality = clamped(eps.value(QStringLiteral("jpegQuality")).toInt(options.epsJpegQuality), 0, 12);
    options.epsHalftone = eps.value(QStringLiteral("halftone")).toBool(options.epsHalftone);
    options.epsTransfer = eps.value(QStringLiteral("transfer")).toBool(options.epsTransfer);
    options.epsTransparentWhites = eps.value(QStringLiteral("transparentWhites")).toBool(options.epsTransparentWhites);
    options.epsPsColorManagement = eps.value(QStringLiteral("psColorManagement")).toBool(options.epsPsColorManagement);
    options.epsVectorData = eps.value(QStringLiteral("vectorData")).toBool(options.epsVectorData);
    options.epsInterpolation = eps.value(QStringLiteral("interpolation")).toBool(options.epsInterpolation);

    const QJsonObject dcs = root.value(QStringLiteral("dcs")).toObject();
    options.dcsVersion = clamped(dcs.value(QStringLiteral("version")).toInt(options.dcsVersion), 1, 2);
    options.dcsPreview = clamped(dcs.value(QStringLiteral("preview")).toInt(options.dcsPreview), 0, 2);

    const QJsonObject raw = root.value(QStringLiteral("raw")).toObject();
    options.rawHeader = raw.value(QStringLiteral("header")).toInt(options.rawHeader);
    options.rawInterleave = clamped(raw.value(QStringLiteral("interleave")).toInt(options.rawInterleave), 0, 1);

    const QJsonObject tga = root.value(QStringLiteral("tga")).toObject();
    options.tgaDepth = clamped(tga.value(QStringLiteral("depth")).toInt(options.tgaDepth), 16, 32);
    options.tgaRle = tga.value(QStringLiteral("rle")).toBool(options.tgaRle);

    const QJsonObject cineon = root.value(QStringLiteral("cineon")).toObject();
    options.cineonBits = clamped(cineon.value(QStringLiteral("bits")).toInt(options.cineonBits), 1, 16);
    return options;
}

int jpegQualityPercent(int quality)
{
    static const int kPercent[13] = {60, 64, 68, 72, 75, 78, 81, 84, 86, 89, 92, 95, 98};
    return kPercent[clamped(quality, 0, 12)];
}

} // namespace pictura
