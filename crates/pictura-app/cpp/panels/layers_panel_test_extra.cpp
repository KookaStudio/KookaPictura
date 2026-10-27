// Extra LayersPanel test seams kept out of layers_panel_test.cpp (its
// 1400-line test budget).

#include "layers_panel.h"
#include "layers_panel_internal.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QModelIndex>
#include <QtGui/QAction>
#include <QtGui/QMouseEvent>
#include <QtWidgets/QMenu>
#include <QtWidgets/QTreeView>

namespace pictura {

QStringList LayersPanel::smartFiltersGroupMenuTextsForTest(const QString& path)
{
    QMenu menu;
    populateSmartFiltersGroupMenu(menu, path);
    QStringList texts;
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator()) {
            texts.push_back(action->text());
        }
    }
    return texts;
}

QStringList LayersPanel::smartFilterRowMenuTextsForTest(const QString& path)
{
    QMenu menu;
    populateSmartFilterMenu(menu, path);
    QStringList texts;
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator()) {
            texts.push_back(action->text());
        }
    }
    return texts;
}

bool LayersPanel::smartFilterMenuEnabledForTest(const QString& path, const QString& text)
{
    QMenu menu;
    if (path.endsWith(QStringLiteral("/@sf"))) {
        populateSmartFiltersGroupMenu(menu, path);
    } else {
        populateSmartFilterMenu(menu, path);
    }
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator() && action->text() == text) {
            return action->isEnabled();
        }
    }
    return false;
}

bool LayersPanel::triggerSmartFilterMenuActionForTest(const QString& path, const QString& text)
{
    QMenu menu;
    if (path.endsWith(QStringLiteral("/@sf"))) {
        populateSmartFiltersGroupMenu(menu, path);
    } else {
        populateSmartFilterMenu(menu, path);
    }
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator() && action->text() == text) {
            action->trigger();
            return true;
        }
    }
    return false;
}

bool LayersPanel::triggerRowMenuActionForPathForTest(const QString& path, const QString& text)
{
    const QModelIndex index = model_ ? model_->indexForPath(path) : QModelIndex();
    QMenu menu;
    populateRowMenu(menu, path, index.data(ColorRole).toInt(), index.data(KindRole).toString(),
                    index.data(LayerRowShapeRole).toBool(),
                    index.data(SmartObjectRole).toBool());
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator() && action->text() == text) {
            action->trigger();
            return true;
        }
    }
    return false;
}

int LayersPanel::rowMenuSeparatorCountForPathForTest(const QString& path)
{
    const QModelIndex index = model_ ? model_->indexForPath(path) : QModelIndex();
    QMenu menu;
    populateRowMenu(menu, path, index.data(ColorRole).toInt(), index.data(KindRole).toString(),
                    index.data(LayerRowShapeRole).toBool(),
                    index.data(SmartObjectRole).toBool());
    int count = 0;
    for (QAction* action : menu.actions()) {
        if (action->isSeparator()) {
            ++count;
        }
    }
    return count;
}

bool LayersPanel::rowMenuHasAdjacentSeparatorsForPathForTest(const QString& path)
{
    const QModelIndex index = model_ ? model_->indexForPath(path) : QModelIndex();
    QMenu menu;
    populateRowMenu(menu, path, index.data(ColorRole).toInt(), index.data(KindRole).toString(),
                    index.data(LayerRowShapeRole).toBool(),
                    index.data(SmartObjectRole).toBool());
    bool previous = false;
    for (QAction* action : menu.actions()) {
        const bool separator = action->isSeparator();
        if (separator && previous) {
            return true;
        }
        previous = separator;
    }
    return false;
}

QString LayersPanel::rowMenuToolTipForPathForTest(const QString& path, const QString& text)
{
    const QModelIndex index = model_ ? model_->indexForPath(path) : QModelIndex();
    QMenu menu;
    populateRowMenu(menu, path, index.data(ColorRole).toInt(), index.data(KindRole).toString(),
                    index.data(LayerRowShapeRole).toBool(),
                    index.data(SmartObjectRole).toBool());
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator() && action->text() == text) {
            return action->toolTip();
        }
    }
    return QString();
}

bool LayersPanel::doubleClickThumbnailForTest(const QString& path)
{
    if (!delegate_ || !tree_) {
        return false;
    }
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid()) {
        return false;
    }
    const QRect vr = tree_->visualRect(index);
    const QPoint pos = delegate_->thumbRect(vr, index).center();
    QMouseEvent dbl(QEvent::MouseButtonDblClick, pos, tree_->viewport()->mapToGlobal(pos),
                    Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &dbl);
    return true;
}

bool LayersPanel::doubleClickMaskForTest(const QString& path)
{
    if (!delegate_ || !tree_) {
        return false;
    }
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid()) {
        return false;
    }
    const QRect vr = tree_->visualRect(index);
    const QRect mask = delegate_->maskThumbRect(vr, index);
    if (mask.isEmpty()) {
        return false;
    }
    const QPoint pos = mask.center();
    QMouseEvent dbl(QEvent::MouseButtonDblClick, pos, tree_->viewport()->mapToGlobal(pos),
                    Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &dbl);
    return true;
}

QRect LayersPanel::rowLinkBadgeRectForTest(const QString& path) const
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !delegate_ || !tree_) {
        return {};
    }
    return delegate_->linkBadgeRect(tree_->visualRect(index), index);
}

void LayersPanel::clickFxButtonForTest(bool alt)
{
    applyFxButton(alt);
}

} // namespace pictura
