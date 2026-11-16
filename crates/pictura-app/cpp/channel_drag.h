#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QMimeData>
#include <QtCore/QString>

namespace pictura {

inline constexpr char kChannelRowMime[] = "application/x-pictura-channel-row";
inline constexpr char kChannelSourceMime[] = "application/x-pictura-channel-source";
inline constexpr char kChannelAlphaMime[] = "application/x-pictura-channel-alpha";
inline constexpr char kChannelMaskMime[] = "application/x-pictura-channel-mask";
inline constexpr char kChannelNameMime[] = "application/x-pictura-channel-name";

struct ChannelDrag {
    const void* source = nullptr;
    int row = -1;
    int alpha = -1;
    int mask = 0x7;
    QString name;
};

inline const void* channelDragSource(const QMimeData* mime)
{
    return mime ? reinterpret_cast<const void*>(static_cast<quintptr>(
                      mime->data(kChannelSourceMime).toULongLong()))
                : nullptr;
}

inline ChannelDrag channelDrag(const QMimeData* mime)
{
    ChannelDrag drag;
    if (!mime || !mime->hasFormat(kChannelRowMime)) {
        return drag;
    }
    drag.source = channelDragSource(mime);
    drag.row = mime->data(kChannelRowMime).toInt();
    drag.alpha = mime->data(kChannelAlphaMime).toInt();
    drag.mask = mime->data(kChannelMaskMime).toInt();
    drag.name = QString::fromUtf8(mime->data(kChannelNameMime));
    return drag;
}

inline QMimeData* makeChannelDragMime(const void* source, int row, int alpha, int mask,
                                      const QString& name)
{
    auto* mime = new QMimeData;
    mime->setData(kChannelRowMime, QByteArray::number(row));
    mime->setData(kChannelSourceMime,
                  QByteArray::number(reinterpret_cast<quintptr>(source)));
    mime->setData(kChannelAlphaMime, QByteArray::number(alpha));
    mime->setData(kChannelMaskMime, QByteArray::number(mask));
    mime->setData(kChannelNameMime, name.toUtf8());
    return mime;
}

} // namespace pictura
