#pragma once

#include <QtCore/QString>
#include <QtCore/QVector>

namespace pictura {

struct LayerRowNode;

/// Command-availability facts the row snapshot does not carry. The panel reads
/// them from the live view when the menu opens, not for every row on refresh.
struct RowFacts {
    bool fillContent = false;
    bool canRasterizeSmartObject = false;
    bool canRelinkSmartObject = false;
    bool canReleaseClipping = false;
    bool hasStyle = false;
    bool canRasterizeStyle = false;
    bool hasMask = false;
    bool hasVectorMask = false;
};

/// One row-menu entry resolved for one row: the table row filtered by the
/// row's kind and state, with its command id and submenu state.
struct ResolvedRowAction {
    QString id;
    QString label;
    QString commandId;
    int block = 0;
    bool enabled = true;
    bool submenu = false;
};

/// The applicable row-menu entries for `row`, in table order. Rename, the
/// Color Label submenu, block separators, and the type/Background menus stay
/// with the panel.
QVector<ResolvedRowAction> resolveRowActions(const LayerRowNode& row, const RowFacts& facts);

} // namespace pictura
