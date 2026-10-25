#pragma once

#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtCore/QVector>
#include <QtGui/QImage>

namespace pictura {
class PictureView;
}

namespace pictura::layers_api {

/// One Layers-panel row, typed. Field-for-field the legacy per-getter output;
/// the style-effect names arrive split. The defaults are the old getters'
/// out-of-range/absent values, so `rowAt` on a bad index reports the same as
/// the deleted `layer_row_*` did.
struct LayerRow {
    QString path;
    int depth = 0;
    QString name;
    QString kind;
    bool visible = false;
    QString blend;
    int opacity = 0;
    int fill = 0;
    int lockBits = 0;
    int color = 0;
    bool clipping = false;
    bool hasMask = false;
    bool maskLinked = false;
    bool maskDisabled = false;
    bool hasVectorMask = false;
    bool vectorMaskLinked = false;
    bool vectorMaskDisabled = false;
    bool hasAdjustment = false;
    QString adjustmentKind;
    bool expandable = false;
    int childCount = 0;
    bool linked = false;
    bool placed = false;
    bool shape = false;
    bool warped = false;
    bool missingFont = false;  // a type layer whose font is not installed
    bool smartObject = false;
    bool hasStyle = false;
    QStringList styleEffects;
    bool hasBlendIf = false;
    QImage thumbnail;
    QImage maskThumbnail;
    QImage vectorMaskThumbnail;
};

/// One crossing per refresh: the rows plus the document extent, the requested
/// thumbnail size, and the view's raster-mask edit target.
struct Snapshot {
    int documentWidth = 0;
    int documentHeight = 0;
    int thumbSize = 0;
    QString maskTargetPath;
    QVector<LayerRow> rows;
};

Snapshot snapshot(PictureView& view, int thumbSize, bool entireDocument);

/// Convenience reads over one `snapshot` per call. Prefer a single `snapshot`
/// (or `rows`) when iterating; calling `rowAt`/`rowCount` inside a loop rebuilds
/// the whole snapshot each iteration.
inline QVector<LayerRow> rows(PictureView& view)
{
    return snapshot(view, 0, false).rows;
}

inline int rowCount(PictureView& view)
{
    return rows(view).size();
}

/// Row `index` in the snapshot's order, or an empty row (empty path) when out
/// of range.
inline LayerRow rowAt(PictureView& view, int index)
{
    const QVector<LayerRow> all = rows(view);
    return index >= 0 && index < all.size() ? all.at(index) : LayerRow{};
}

/// The first row whose path matches, or an empty row (empty path) when absent.
inline LayerRow rowForPath(PictureView& view, const QString& path)
{
    for (const LayerRow& row : rows(view)) {
        if (row.path == path) {
            return row;
        }
    }
    return LayerRow{};
}

} // namespace pictura::layers_api
