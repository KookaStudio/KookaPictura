#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QString>
#include <QtCore/QVector>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtCore/QSize>

#include <cstdint>

#include "api/engine_bridge.h"

namespace pictura {

// One export cell resolved from the document's user/auto slices.
struct ExportSlice {
    int x = 0;
    int y = 0;
    int width = 0;
    int height = 0;
    int number = 0;
    int userIndex = -1;
};

// One named export image, in target-image pixels.
struct SlicePlacement {
    QString name;
    int x = 0;
    int y = 0;
    int width = 0;
    int height = 0;
};

// One pane's Save for Web settings.
struct WebSettings {
    int format = 0;
    int reduction = 1;
    int colors = 128;
    int dither = 1;
    int ditherAmount = 88;
    bool transparency = true;
    bool mappedTransparency = false;
    bool interlaced = false;
    int matte = 0;
    int quality = 60;
    bool progressive = false;
    bool optimizedJpeg = true;
    bool pngTransparency = true;
    int wbmpDither = 1;
    int wbmpAmount = 88;
    QVector<QRgb> locked;
    QVector<QRgb> custom;
    // An index-preserving table edit: the packed indexed image the dialog
    // last produced. It is valid only while its size matches the image.
    QByteArray packed;
};

WebPalette paletteOf(const WebSettings& settings);

// The active pane's optimised bytes for `image`, without metadata.
QByteArray encodeWebImage(const QImage& image, const WebSettings& settings);

// The packed indexed result (count + RGBA entries + indices) for GIF/PNG-8.
QByteArray quantizeWebImage(const QImage& image, const WebSettings& settings);

// A packed indexed result as a Qt indexed image, ready for the PNG writer.
QImage indexedImageFromPacked(const QByteArray& packed, const QSize& size);

QVector<QRgb> paletteFromPacked(const QByteArray& packed);

QString webSuffix(const WebSettings& settings);

QByteArray embedWebMetadata(const QByteArray& data, const QString& suffix,
                            const WebMetadata& raw, int level);

// A lower-quality variant for 4-Up Repopulate: JPEG quality steps down, a
// palette table shrinks.
WebSettings loweredSettings(const WebSettings& settings, int step);

} // namespace pictura
