#include "save_format_options.h"

#include "api/engine_bridge.h"

#include <QtCore/QJsonValue>
#include <QtCore/QtGlobal>

namespace pictura {

namespace {

int clamped(int value, int low, int high)
{
    return value < low ? low : (value > high ? high : value);
}

} // namespace

const QVector<SaveFormatRow>& saveFormatRows()
{
    static const QVector<SaveFormatRow> rows = {
        {0, SaveFormatKind::None, QStringLiteral("psd"),
         QT_TRANSLATE_NOOP("PicturaFiles", "Photoshop"), QStringLiteral(" (*.PSD)"),
         QStringLiteral(".psd")},
        {1, SaveFormatKind::None, QStringLiteral("psb"),
         QT_TRANSLATE_NOOP("PicturaFiles", "Photoshop Large Format"), QStringLiteral(" (*.PSB)"),
         QStringLiteral(".psb")},
        {2, SaveFormatKind::Png, QStringLiteral("png"),
         QT_TRANSLATE_NOOP("PicturaFiles", "PNG"), QStringLiteral(" (*.PNG)"),
         QStringLiteral(".png")},
        {3, SaveFormatKind::Jpeg, QStringLiteral("jpeg"),
         QT_TRANSLATE_NOOP("PicturaFiles", "JPEG"), QStringLiteral(" (*.JPG *.JPEG *.JPE)"),
         QStringLiteral(".jpg")},
        {4, SaveFormatKind::Tiff, QStringLiteral("tiff"),
         QT_TRANSLATE_NOOP("PicturaFiles", "TIFF"), QStringLiteral(" (*.TIF *.TIFF)"),
         QStringLiteral(".tif")},
        {5, SaveFormatKind::None, QStringLiteral("webp"),
         QT_TRANSLATE_NOOP("PicturaFiles", "WebP"), QStringLiteral(" (*.WEBP)"),
         QStringLiteral(".webp")},
        {6, SaveFormatKind::Bmp, QStringLiteral("bmp"),
         QT_TRANSLATE_NOOP("PicturaFiles", "BMP"), QStringLiteral(" (*.BMP)"),
         QStringLiteral(".bmp")},
        {7, SaveFormatKind::None, QStringLiteral("pdf"),
         QT_TRANSLATE_NOOP("PicturaFiles", "Photoshop PDF"), QStringLiteral(" (*.PDF *.PDP)"),
         QStringLiteral(".pdf")},
        {8, SaveFormatKind::Gif, QStringLiteral("gif"),
         QT_TRANSLATE_NOOP("PicturaFiles", "CompuServe GIF"), QStringLiteral(" (*.GIF)"),
         QStringLiteral(".gif")},
        {9, SaveFormatKind::Eps, QStringLiteral("eps"),
         QT_TRANSLATE_NOOP("PicturaFiles", "Photoshop EPS"), QStringLiteral(" (*.EPS)"),
         QStringLiteral(".eps")},
        {10, SaveFormatKind::Dcs, QStringLiteral("dcs"),
         QT_TRANSLATE_NOOP("PicturaFiles", "Photoshop DCS"), QStringLiteral(" (*.EPS)"),
         QStringLiteral(".eps")},
        {11, SaveFormatKind::Raw, QStringLiteral("raw"),
         QT_TRANSLATE_NOOP("PicturaFiles", "Photoshop Raw"), QStringLiteral(" (*.RAW)"),
         QStringLiteral(".raw")},
        {12, SaveFormatKind::Tga, QStringLiteral("tga"),
         QT_TRANSLATE_NOOP("PicturaFiles", "Targa"), QStringLiteral(" (*.TGA *.VDA *.ICB *.VST)"),
         QStringLiteral(".tga")},
        {13, SaveFormatKind::Cineon, QStringLiteral("cineon"),
         QT_TRANSLATE_NOOP("PicturaFiles", "Cineon"), QStringLiteral(" (*.CIN)"),
         QStringLiteral(".cin")},
    };
    return rows;
}

int saveFormatIdForSuffix(const QString& suffix)
{
    const QByteArray utf8 = suffix.toUtf8();
    return pictura::save_format_id_for_suffix(
        ::rust::Str(utf8.constData(), static_cast<size_t>(utf8.size())));
}

int saveFormatIdForSourceName(const QString& name)
{
    const QByteArray utf8 = name.toUtf8();
    return pictura::save_format_id_for_source_name(
        ::rust::Str(utf8.constData(), static_cast<size_t>(utf8.size())));
}

int saveFormatIdForKind(SaveFormatKind kind)
{
    if (kind == SaveFormatKind::None) {
        return -1;
    }
    for (const SaveFormatRow& row : saveFormatRows()) {
        if (row.kind == kind) {
            return row.id;
        }
    }
    return -1;
}

int saveFormatFilterIndexForId(int id)
{
    const QVector<SaveFormatRow>& rows = saveFormatRows();
    for (int i = 0; i < rows.size(); ++i) {
        if (rows.at(i).id == id) {
            return i;
        }
    }
    return -1;
}

quint32 saveFormatFlags(int id) { return pictura::save_format_flags(id); }

bool saveFormatHasFlag(int id, quint32 flag)
{
    return id >= 0 && (saveFormatFlags(id) & flag) != 0;
}

bool saveFormatIsNativeSuffix(const QString& suffix)
{
    return saveFormatHasFlag(saveFormatIdForSuffix(suffix), kSaveFormatNative);
}

bool saveFormatKeepsLayers(const QString& suffix)
{
    return saveFormatHasFlag(saveFormatIdForSuffix(suffix), kSaveFormatKeepsLayers);
}

bool saveFormatOffersProfile(const QString& suffix)
{
    return saveFormatHasFlag(saveFormatIdForSuffix(suffix), kSaveFormatOffersProfile);
}

bool saveFormatIsRasterSuffix(const QString& suffix)
{
    return saveFormatHasFlag(saveFormatIdForSuffix(suffix), kSaveFormatRaster);
}

bool saveFormatIsPdfSuffix(const QString& suffix)
{
    return saveFormatHasFlag(saveFormatIdForSuffix(suffix), kSaveFormatPdf);
}

SaveFormatKind saveFormatKindForSuffix(const QString& suffix)
{
    const int index = saveFormatFilterIndexForId(saveFormatIdForSuffix(suffix));
    return index < 0 ? SaveFormatKind::None : saveFormatRows().at(index).kind;
}

SaveFormatKind saveFormatKindForFilter(int filterIndex)
{
    const QVector<SaveFormatRow>& rows = saveFormatRows();
    return filterIndex >= 0 && filterIndex < rows.size() ? rows.at(filterIndex).kind
                                                         : SaveFormatKind::None;
}

QString saveFormatKey(SaveFormatKind kind)
{
    if (kind == SaveFormatKind::None) {
        return QString();
    }
    for (const SaveFormatRow& row : saveFormatRows()) {
        if (row.kind == kind) {
            return row.name;
        }
    }
    return QString();
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
    gif.insert(QStringLiteral("palette"), options.gifPalette);
    gif.insert(QStringLiteral("colors"), options.gifColors);
    gif.insert(QStringLiteral("dither"), options.gifDither);
    gif.insert(QStringLiteral("amount"), options.gifAmount);

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
    options.gifPalette = clamped(gif.value(QStringLiteral("palette")).toInt(options.gifPalette), 0, 4);
    options.gifColors = clamped(gif.value(QStringLiteral("colors")).toInt(options.gifColors), 2, 256);
    options.gifDither = clamped(gif.value(QStringLiteral("dither")).toInt(options.gifDither), 0, 3);
    options.gifAmount = clamped(gif.value(QStringLiteral("amount")).toInt(options.gifAmount), 0, 100);

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
