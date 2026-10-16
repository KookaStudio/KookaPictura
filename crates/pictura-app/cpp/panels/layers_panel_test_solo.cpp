// The mask solo test seam: synthesize an Alt-click on a row's mask thumbnail,
// routed through the same event filter a real user click takes. Kept in its own
// translation unit so layers_panel_test.cpp stays under its size cap.

#include "layers_panel.h"

#include "layers_panel_internal.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QModelIndex>
#include <QtCore/QPoint>
#include <QtCore/QRect>

#include <QtGui/QMouseEvent>

#include <QtWidgets/QTreeView>

namespace pictura {

bool LayersPanel::clickMaskThumbWithModifiersForTest(const QString& path,
                                                     Qt::KeyboardModifiers mods)
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !delegate_ || !tree_) {
        return false;
    }
    const QRect mask = delegate_->maskThumbRect(tree_->visualRect(index), index);
    if (mask.isEmpty()) {
        return false;
    }
    const QPoint at = mask.center();
    QMouseEvent press(QEvent::MouseButtonPress, at, tree_->viewport()->mapToGlobal(at),
                      Qt::LeftButton, Qt::LeftButton, mods);
    QCoreApplication::sendEvent(tree_->viewport(), &press);
    return true;
}

bool LayersPanel::altClickMaskThumbnailForTest(const QString& path)
{
    return clickMaskThumbWithModifiersForTest(path, Qt::AltModifier);
}

bool LayersPanel::altShiftClickMaskThumbnailForTest(const QString& path)
{
    return clickMaskThumbWithModifiersForTest(path, Qt::AltModifier | Qt::ShiftModifier);
}

} // namespace pictura
