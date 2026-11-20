#pragma once

// A minimal camera raw fixture for the raw open tests (#400): a little-endian,
// single-IFD, uncompressed 16-bit RGGB DNG with a smooth ramp. With colour, its
// ColorMatrix1 is XYZ-D50-to-linear-ProPhoto at D50, so the camera's native
// RGB is ProPhoto; without, the codec refuses its camera colour.

#include <QtCore/QByteArray>
#include <QtCore/QList>
#include <QtCore/QtEndian>

#include <algorithm>
#include <cmath>

namespace pictura::test {

namespace raw_fixture_detail {

struct Entry {
    quint16 tag;
    quint16 type;
    quint32 count;
    QByteArray bytes;
};

inline QByteArray le16(quint16 v)
{
    QByteArray out(2, '\0');
    qToLittleEndian(v, out.data());
    return out;
}

inline QByteArray le32(quint32 v)
{
    QByteArray out(4, '\0');
    qToLittleEndian(v, out.data());
    return out;
}

inline Entry shorts(quint16 tag, const QList<quint16>& values)
{
    QByteArray bytes;
    for (quint16 v : values) {
        bytes += le16(v);
    }
    return {tag, 3, quint32(values.size()), bytes};
}

inline Entry longs(quint16 tag, const QList<quint32>& values)
{
    QByteArray bytes;
    for (quint32 v : values) {
        bytes += le32(v);
    }
    return {tag, 4, quint32(values.size()), bytes};
}

inline Entry rawBytes(quint16 tag, const QByteArray& values)
{
    return {tag, 1, quint32(values.size()), values};
}

} // namespace raw_fixture_detail

inline QByteArray rawDng(int width, int height, bool withColour)
{
    using namespace raw_fixture_detail;
    QByteArray samples;
    for (int y = 0; y < height; ++y) {
        for (int x = 0; x < width; ++x) {
            samples += le16(quint16(8000 + x * 40000 / width + y * 9000 / height));
        }
    }
    const quint32 stripAt = 8;
    const quint32 ifdAt = stripAt + quint32(samples.size());
    QList<Entry> entries = {
        longs(254, {0}),
        longs(256, {quint32(width)}),
        longs(257, {quint32(height)}),
        shorts(258, {16}),
        shorts(259, {1}),
        shorts(262, {32803}),
        longs(273, {stripAt}),
        shorts(277, {1}),
        longs(278, {quint32(height)}),
        longs(279, {quint32(samples.size())}),
        shorts(33421, {2, 2}),
        rawBytes(33422, QByteArray("\x00\x01\x01\x02", 4)),
        rawBytes(50706, QByteArray("\x01\x04\x00\x00", 4)),
        {50708, 2, 9, QByteArray("Test Cam\0", 9)},
        {50728, 5, 3, le32(1) + le32(1) + le32(1) + le32(1) + le32(1) + le32(1)},
        shorts(50778, {23}),
    };
    if (withColour) {
        // XYZ (D50) to linear ProPhoto (ROMM).
        const double m[9] = {1.3459433, -0.2556075, -0.0511118, -0.5445989, 1.5081673,
                             0.0205351, 0.0000000,  0.0000000,  1.2118128};
        QByteArray bytes;
        for (double v : m) {
            bytes += le32(quint32(qint32(std::lround(v * 10000.0))));
            bytes += le32(10000);
        }
        entries.append({50721, 10, 9, bytes});
    }
    std::sort(entries.begin(), entries.end(),
              [](const Entry& a, const Entry& b) { return a.tag < b.tag; });

    QByteArray out("II", 2);
    out += le16(42);
    out += le32(ifdAt);
    out += samples;
    quint32 blobAt = ifdAt + 2 + quint32(entries.size()) * 12 + 4;
    QByteArray table = le16(quint16(entries.size()));
    QByteArray blobs;
    for (const Entry& entry : entries) {
        table += le16(entry.tag) + le16(entry.type) + le32(entry.count);
        if (entry.bytes.size() <= 4) {
            table += entry.bytes + QByteArray(4 - entry.bytes.size(), '\0');
        } else {
            table += le32(blobAt);
            blobAt += quint32(entry.bytes.size());
            blobs += entry.bytes;
        }
    }
    table += le32(0);
    return out + table + blobs;
}

} // namespace pictura::test
