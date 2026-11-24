#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QMimeData>
#include <QtCore/QObject>
#include <QtCore/QPointer>
#include <QtCore/QString>

namespace pictura {

inline constexpr char kChannelRowMime[] = "application/x-pictura-channel-row";
inline constexpr char kChannelSourceMime[] = "application/x-pictura-channel-source";
inline constexpr char kChannelAlphaMime[] = "application/x-pictura-channel-alpha";
inline constexpr char kChannelMaskMime[] = "application/x-pictura-channel-mask";
inline constexpr char kChannelNameMime[] = "application/x-pictura-channel-name";

struct ChannelDrag {
    QPointer<QObject> sourceGuard;
    int row = -1;
    int alpha = -1;
    int mask = 0x7;
    QString name;
};

class ChannelDragMime : public QMimeData {
public:
    explicit ChannelDragMime(QObject* source)
        : sourceGuard(source)
    {
    }

    QPointer<QObject> sourceGuard;
};

inline ChannelDrag channelDrag(const QMimeData* mime)
{
    ChannelDrag drag;
    if (!mime || !mime->hasFormat(kChannelRowMime)) {
        return drag;
    }
    if (const auto* guarded = dynamic_cast<const ChannelDragMime*>(mime)) {
        drag.sourceGuard = guarded->sourceGuard;
    }
    drag.row = mime->data(kChannelRowMime).toInt();
    drag.alpha = mime->data(kChannelAlphaMime).toInt();
    drag.mask = mime->data(kChannelMaskMime).toInt();
    drag.name = QString::fromUtf8(mime->data(kChannelNameMime));
    return drag;
}

inline QMimeData* makeChannelDragMime(QObject* source, int row, int alpha, int mask,
                                       const QString& name)
{
    auto* mime = new ChannelDragMime(source);
    mime->setData(kChannelRowMime, QByteArray::number(row));
    mime->setData(kChannelSourceMime, QByteArray());
    mime->setData(kChannelAlphaMime, QByteArray::number(alpha));
    mime->setData(kChannelMaskMime, QByteArray::number(mask));
    mime->setData(kChannelNameMime, name.toUtf8());
    return mime;
}

} // namespace pictura
