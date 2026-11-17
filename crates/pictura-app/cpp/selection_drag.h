#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QMimeData>

namespace pictura {

inline constexpr char kSelectionSourceMime[] = "application/x-pictura-selection-source";

inline const void* selectionDragSource(const QMimeData* mime)
{
    return mime ? reinterpret_cast<const void*>(static_cast<quintptr>(
                      mime->data(kSelectionSourceMime).toULongLong()))
                : nullptr;
}

inline QMimeData* makeSelectionDragMime(const void* source)
{
    auto* mime = new QMimeData;
    mime->setData(kSelectionSourceMime,
                  QByteArray::number(reinterpret_cast<quintptr>(source)));
    return mime;
}

} // namespace pictura
