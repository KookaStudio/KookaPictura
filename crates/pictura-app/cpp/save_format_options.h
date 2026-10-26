#pragma once

#include <QtCore/QJsonObject>
#include <QtCore/QString>

namespace pictura {

// Which per-format options dialog a Save As destination opens.
enum class SaveFormatKind { None, Jpeg, Png, Bmp, Tiff };

// The options kind for a path suffix (without the dot, any case); None for a
// format that opens no format options dialog.
SaveFormatKind saveFormatKindForSuffix(const QString& suffix);

// The session-store key for a kind ("jpeg"/"png"/"bmp"/"tiff"), or empty.
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
    int pngCompression = 0;    // 0 None/Fast ... 100 Smallest/Slow (Qt's 0-100)
    int pngInterlace = 0;      // 0 None, 1 Interlaced (shown, not honored)

    // BMP Options.
    int bmpFormat = 0;         // 0 Windows, 1 OS/2
    int bmpDepth = 24;         // 1/4/8/16/24/32 (shown, not honored)
    bool bmpRle = false;
    bool bmpFlip = false;

    // TIFF Options.
    int tiffCompression = 1;   // Qt TIFF enum: 0 None, 1 LZW, 5 JPEG (RLE/ZIP unhonored)
    int tiffJpegQuality = 10;  // 0-12 when compression is JPEG
    int tiffPixelOrder = 0;    // 0 Interleaved, 1 Per Channel (shown, not honored)
    int tiffByteOrder = 0;     // 0 IBM PC, 1 Macintosh (shown, not honored)
    int tiffLayerCompression = 0;  // 0 RLE, 1 ZIP, 2 Discard (shown, not honored)
    bool tiffPyramid = false;
    bool tiffTransparency = true;
};

// The whole value as a session object keyed by "jpeg"/"png"/"bmp"/"tiff".
QJsonObject saveFormatOptionsToJson(const SaveFormatOptions& options);

// Read the value back; an absent key or field keeps the CS6 default.
SaveFormatOptions saveFormatOptionsFromJson(const QJsonObject& root);

// The JPEG quality (0-12) as Qt's 0-100 percent, clamped to at least 1.
int jpegQualityPercent(int quality);

} // namespace pictura
