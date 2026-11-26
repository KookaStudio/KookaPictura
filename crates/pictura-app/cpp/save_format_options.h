#pragma once

#include <QtCore/QJsonObject>
#include <QtCore/QString>
#include <QtCore/QVector>
#include <QtCore/QtGlobal>

namespace pictura {

// Which per-format options dialog a Save As or Save a Copy destination opens.
// The integer values are the wire codes shared with the Rust SaveKind.
enum class SaveFormatKind {
    None = 0,
    Jpeg = 1,
    Png = 2,
    Bmp = 3,
    Tiff = 4,
    Gif = 5,
    Eps = 6,
    Dcs = 7,
    Raw = 8,
    Tga = 9,
    Cineon = 10,
    Pict = 11,
};

// The options kind for a path suffix (without the dot, any case); None for a
// format that opens no format options dialog.
SaveFormatKind saveFormatKindForSuffix(const QString& suffix);

// One displayed Save As format. The table order is the filter order. `id` is
// the engine catalog ordinal, never the options wire code.
struct SaveFormatRow {
    int id;
    SaveFormatKind kind;
    QString name;
    const char* label;
    QString pattern;
    QString canonicalSuffix;
};

// Policy flags mirrored from the engine catalog.
constexpr quint32 kSaveFormatNative = 1u << 0;
constexpr quint32 kSaveFormatPdf = 1u << 1;
constexpr quint32 kSaveFormatRaster = 1u << 2;
constexpr quint32 kSaveFormatKeepsLayers = 1u << 3;
constexpr quint32 kSaveFormatOffersProfile = 1u << 4;

// The 14 Save As formats in display order; "All Formats" is not a row.
const QVector<SaveFormatRow>& saveFormatRows();

// A catalog ordinal for a typed suffix (alias-only, so `.dcs` is unknown) or a
// remembered source name (`"dcs"`), or -1.
int saveFormatIdForSuffix(const QString& suffix);
int saveFormatIdForSourceName(const QString& name);
// The ordinal for an options kind, or -1 when it names no format.
int saveFormatIdForKind(SaveFormatKind kind);
// The display index for a catalog ordinal, or -1.
int saveFormatFilterIndexForId(int id);
quint32 saveFormatFlags(int id);
bool saveFormatHasFlag(int id, quint32 flag);

// Policy predicates derived from the catalog flags.
bool saveFormatIsNativeSuffix(const QString& suffix);
bool saveFormatKeepsLayers(const QString& suffix);
bool saveFormatOffersProfile(const QString& suffix);
bool saveFormatIsRasterSuffix(const QString& suffix);
bool saveFormatIsPdfSuffix(const QString& suffix);

// The options kind for a Save As filter index (index into saveFileFilters()).
SaveFormatKind saveFormatKindForFilter(int filterIndex);

// The session-store key for a kind ("jpeg"/"png"/...), or empty.
QString saveFormatKey(SaveFormatKind kind);

// The remembered per-format settings. One flat value; the session store keeps
// one object per format and this struct is the union of their fields.
struct SaveFormatOptions {
    // JPEG Options.
    int jpegQuality = 10;      // 0-12 (CS6 scale)
    int jpegFormat = 0;        // 0 Baseline, 1 Baseline Optimized, 2 Progressive
    int jpegScans = 3;         // progressive scan count (shown, not honored)
    int jpegMatte = -1;        // packed 0xRRGGBB, -1 = None
    bool jpegPreview = true;

    // PNG Options.
    int pngCompression = 0;    // 0 None/Fast ... 100 Smallest/Slow
    int pngInterlace = 0;      // 0 None, 1 Interlaced

    // BMP Options.
    int bmpFormat = 0;         // 0 Windows, 1 OS/2
    int bmpDepth = 24;         // 1/4/8/16/24/32
    bool bmpRle = false;
    bool bmpFlip = false;
    int bmpAdvanced = 0;       // 0 X1R5G5B5, 1 A1R5G5B5, 2 R5G6B5

    // TIFF Options.
    int tiffCompression = 2;   // 0 None, 1 RLE, 2 LZW, 3 ZIP, 4 JPEG
    int tiffJpegQuality = 10;  // 0-12 when compression is JPEG
    int tiffPixelOrder = 0;    // 0 Interleaved, 1 Per Channel
    int tiffByteOrder = 0;     // 0 IBM PC, 1 Macintosh
    int tiffLayerCompression = 0;  // 0 RLE, 1 ZIP, 2 Discard
    bool tiffPyramid = false;
    bool tiffTransparency = true;

    // GIF Options.
    int gifRowOrder = 0;       // 0 Normal, 1 Interlaced
    int gifPalette = 2;        // 0 Exact, 1 Web, 2/3/4 Local Perceptual/Selective/Adaptive
    int gifColors = 256;
    int gifDither = 1;         // 0 None, 1 Diffusion, 2 Pattern, 3 Noise
    int gifAmount = 75;

    // Photoshop EPS Options.
    int epsPreview = 2;        // 0 None, 1 TIFF 1-bit, 2 TIFF 8-bit
    int epsEncoding = 2;       // 0 ASCII, 1 ASCII85, 2 Binary, 3 JPEG
    int epsJpegQuality = 10;
    bool epsHalftone = false;
    bool epsTransfer = false;
    bool epsTransparentWhites = false;
    bool epsPsColorManagement = false;
    bool epsVectorData = false;
    bool epsInterpolation = true;

    // DCS Format.
    int dcsVersion = 2;        // 1.0 or 2.0
    int dcsPreview = 2;        // as EPS preview

    // Photoshop Raw Options.
    int rawHeader = 0;
    int rawInterleave = 0;     // 0 Interleaved, 1 Non-interleaved

    // Targa Options.
    int tgaDepth = 24;         // 16/24/32
    bool tgaRle = false;

    // Cineon Options (fixed).
    int cineonBits = 10;

    // PICT Options.
    int pictResolution = 32;   // 16 or 32 bits per pixel
    int pictCompression = 0;   // 0 None, 1 JPEG Low, 2 JPEG Medium, 3 JPEG High, 4 JPEG Maximum
};

// The whole value as a session object keyed by the per-format keys.
QJsonObject saveFormatOptionsToJson(const SaveFormatOptions& options);

// Read the value back; an absent key or field keeps the CS6 default.
SaveFormatOptions saveFormatOptionsFromJson(const QJsonObject& root);

// The JPEG quality (0-12) as Qt's 0-100 percent, clamped to at least 1.
int jpegQualityPercent(int quality);

} // namespace pictura
