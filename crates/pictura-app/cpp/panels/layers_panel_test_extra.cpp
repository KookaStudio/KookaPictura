// Extra LayersPanel test seams kept out of layers_panel_test.cpp (its
// 1400-line test budget).

#include "layers_panel.h"
#include "layers_panel_internal.h"

#include "api/engine_bridge.h"
#include "commands.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QModelIndex>
#include <QtGui/QAction>
#include <QtGui/QMouseEvent>
#include <QtWidgets/QMenu>
#include <QtWidgets/QTreeView>

namespace pictura {

bool LayersPanel::performRowActionForTest(const QString& id, const QString& path)
{
    if (!registry_ || !view_) {
        return false;
    }
    if (id == QStringLiteral("relinkToFile")) {
        if (relink_source_id(*view_, path).isEmpty()) {
            return false;
        }
        makeRowAnchor(path);
        emit smartObjectContentsRequested(QStringLiteral("relink"), path);
        return true;
    }
    static const QHash<QString, const char*> kCommands = {
        {QStringLiteral("moveUp"), command_ids::PanelLayersMoveUp},
        {QStringLiteral("moveDown"), command_ids::PanelLayersMoveDown},
        {QStringLiteral("convertToSmartObject"), command_ids::LayerSmartObjectConvertTo},
        {QStringLiteral("editContents"), command_ids::LayerSmartObjectEditContents},
        {QStringLiteral("deleteLayerMask"), command_ids::LayerMaskDelete},
        {QStringLiteral("clearLayerStyle"), command_ids::LayerStyleClearLayerStyle},
    };
    const auto it = kCommands.find(id);
    if (it == kCommands.end()) {
        return false;
    }
    makeRowAnchor(path);
    return registry_->dispatch(QString::fromLatin1(it.value()));
}

const LayerRowNode* LayersPanel::menuRowForTest(const QString& path)
{
    const LayerRowNode* row = model_ ? model_->rowForPath(path) : nullptr;
    if (!row && model_) {
        refresh();
        row = model_->rowForPath(path);
    }
    return row;
}

LayerRowNode syntheticRow(const QString& kind, bool shape, bool smart)
{
    LayerRowNode row;
    row.kind = kind;
    row.shape = shape;
    row.smartObject = smart;
    return row;
}

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
    const LayerRowNode* row = menuRowForTest(path);
    if (!row) {
        return false;
    }
    QMenu menu;
    populateRowMenu(menu, path, row->color, *row);
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
    const LayerRowNode* row = menuRowForTest(path);
    if (!row) {
        return 0;
    }
    QMenu menu;
    populateRowMenu(menu, path, row->color, *row);
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
    const LayerRowNode* row = menuRowForTest(path);
    if (!row) {
        return false;
    }
    QMenu menu;
    populateRowMenu(menu, path, row->color, *row);
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
    const LayerRowNode* row = menuRowForTest(path);
    if (!row) {
        return QString();
    }
    QMenu menu;
    populateRowMenu(menu, path, row->color, *row);
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

QString LayersPanel::rowKindForTest(const QString& path) const
{
    const QModelIndex index = model_ ? model_->indexForPath(path) : QModelIndex();
    return index.data(KindRole).toString();
}

QString LayersPanel::rowNameForTest(const QString& path) const
{
    const QModelIndex index = model_ ? model_->indexForPath(path) : QModelIndex();
    return index.data(Qt::DisplayRole).toString();
}

bool LayersPanel::rowVisibleForTest(const QString& path) const
{
    const QModelIndex index = model_ ? model_->indexForPath(path) : QModelIndex();
    return index.isValid() && index.data(VisibleRole).toBool();
}

bool LayersPanel::rowExistsForTest(const QString& path) const
{
    return model_ && model_->indexForPath(path).isValid();
}

bool LayersPanel::rowExpandedForTest(const QString& path) const
{
    const QModelIndex index = proxyIndexForPath(path);
    return index.isValid() && tree_ && tree_->isExpanded(index);
}

bool LayersPanel::clickListToggleForTest(const QString& path)
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !tree_ || !delegate_) {
        return false;
    }
    const QRect vr = tree_->visualRect(index);
    const QRect toggle = delegate_->listToggleRect(vr, index);
    if (toggle.isEmpty()) {
        return false;
    }
    const QPoint at = toggle.center();
    QMouseEvent press(QEvent::MouseButtonPress, at, tree_->viewport()->mapToGlobal(at),
                      Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &press);
    return true;
}

QStringList LayersPanel::dragPathsForSyntheticForTest(const QString& path)
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !tree_ || !delegate_) {
        return {};
    }
    const QRect vr = tree_->visualRect(index);
    const QPoint at = delegate_->nameRect(vr, index).center();
    QMouseEvent press(QEvent::MouseButtonPress, at, tree_->viewport()->mapToGlobal(at),
                      Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &press);
    const QStringList paths = tree_->dragPathsForTest();
    QMouseEvent release(QEvent::MouseButtonRelease, at, tree_->viewport()->mapToGlobal(at),
                        Qt::LeftButton, Qt::NoButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &release);
    return paths;
}

bool LayersPanel::clickRowEyeForTest(const QString& path)
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !tree_ || !delegate_) {
        return false;
    }
    const QRect vr = tree_->visualRect(index);
    const QRect eye = delegate_->eyeRect(QRect(0, 0, vr.width(), vr.height()), index);
    const QPointF at(vr.left() + eye.center().x(), vr.center().y());
    QMouseEvent press(QEvent::MouseButtonPress, at, tree_->viewport()->mapToGlobal(at),
                      Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &press);
    return true;
}

bool LayersPanel::clickRowForTest(const QString& path)
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !tree_ || !delegate_) {
        return false;
    }
    const QRect vr = tree_->visualRect(index);
    if (vr.isEmpty()) {
        return false;
    }
    const QPointF at = delegate_->nameRect(vr, index).center();
    QMouseEvent press(QEvent::MouseButtonPress, at, tree_->viewport()->mapToGlobal(at),
                      Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &press);
    QMouseEvent release(QEvent::MouseButtonRelease, at, tree_->viewport()->mapToGlobal(at),
                        Qt::LeftButton, Qt::NoButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &release);
    return true;
}

QStringList LayersPanel::layerStyleMenuTextsForTest(const QString& path)
{
    QMenu menu;
    populateLayerStyleMenu(menu, path);
    QStringList texts;
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator()) {
            texts.push_back(action->text());
        }
    }
    return texts;
}

bool LayersPanel::layerStyleMenuEnabledForTest(const QString& path, const QString& text)
{
    QMenu menu;
    populateLayerStyleMenu(menu, path);
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator() && action->text() == text) {
            return action->isEnabled();
        }
    }
    return false;
}

bool LayersPanel::triggerLayerStyleMenuActionForTest(const QString& path, const QString& text)
{
    QMenu menu;
    populateLayerStyleMenu(menu, path);
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator() && action->text() == text) {
            action->trigger();
            return true;
        }
    }
    return false;
}

bool LayersPanel::doubleClickFxBadgeForTest(const QString& path)
{
    if (!delegate_ || !tree_) {
        return false;
    }
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid()) {
        return false;
    }
    const QRect vr = tree_->visualRect(index);
    const QRect fx = delegate_->fxRect(vr, index);
    if (fx.isEmpty()) {
        return false;
    }
    const QPoint pos = fx.center();
    QMouseEvent dbl(QEvent::MouseButtonDblClick, pos, tree_->viewport()->mapToGlobal(pos),
                    Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &dbl);
    return true;
}

bool LayersPanel::doubleClickRowCenterForTest(const QString& path)
{
    if (!tree_) {
        return false;
    }
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid()) {
        return false;
    }
    const QPoint pos = tree_->visualRect(index).center();
    QMouseEvent dbl(QEvent::MouseButtonDblClick, pos, tree_->viewport()->mapToGlobal(pos),
                    Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &dbl);
    return true;
}

bool LayersPanel::doubleClickRightOfNameForTest(const QString& path)
{
    if (!delegate_ || !tree_) {
        return false;
    }
    clearStaleEditorsForTest();
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid()) {
        return false;
    }
    const QRect vr = tree_->visualRect(index);
    const QPoint pos(qMin(vr.right(), delegate_->nameRect(vr, index).right() + 8),
                     vr.center().y());
    QMouseEvent dbl(QEvent::MouseButtonDblClick, pos, tree_->viewport()->mapToGlobal(pos),
                    Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &dbl);
    return true;
}

QStringList LayersPanel::rowMenuCommandIdsForPathForTest(const QString& path)
{
    makeRowAnchor(path);
    const LayerRowNode* row = menuRowForTest(path);
    if (!row) {
        return {};
    }
    QMenu menu;
    populateRowMenu(menu, path, row->color, *row);
    QStringList ids;
    for (QAction* action : menu.actions()) {
        const QString id = action->data().toString();
        if (!id.isEmpty()) {
            ids.push_back(id);
        }
    }
    return ids;
}


QStringList LayersPanel::rowMenuSubmenuTextsForPathForTest(const QString& path,
                                                           const QString& submenuText)
{
    makeRowAnchor(path);
    const LayerRowNode* row = menuRowForTest(path);
    if (!row) {
        return {};
    }
    QMenu menu;
    populateRowMenu(menu, path, row->color, *row);
    for (QAction* action : menu.actions()) {
        if (action->menu() && action->text() == submenuText) {
            QStringList texts;
            for (QAction* item : action->menu()->actions()) {
                if (!item->isSeparator()) {
                    texts.push_back(item->text());
                }
            }
            return texts;
        }
    }
    return {};
}

bool LayersPanel::rowMenuSubmenuItemEnabledForPathForTest(const QString& path,
                                                          const QString& submenuText,
                                                          const QString& itemText)
{
    makeRowAnchor(path);
    const LayerRowNode* row = menuRowForTest(path);
    if (!row) {
        return false;
    }
    QMenu menu;
    populateRowMenu(menu, path, row->color, *row);
    for (QAction* action : menu.actions()) {
        if (!action->menu() || action->text() != submenuText) {
            continue;
        }
        for (QAction* item : action->menu()->actions()) {
            if (!item->isSeparator() && item->text() == itemText) {
                return item->isEnabled();
            }
        }
    }
    return false;
}


QStringList LayersPanel::rowMenuTextsForPathForTest(const QString& path)
{
    makeRowAnchor(path);
    const LayerRowNode* row = menuRowForTest(path);
    if (!row) {
        return {};
    }
    QMenu menu;
    populateRowMenu(menu, path, row->color, *row);
    QStringList texts;
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator()) {
            texts.push_back(action->text());
        }
    }
    return texts;
}

bool LayersPanel::rowMenuEnabledForPathForTest(const QString& path, const QString& text)
{
    makeRowAnchor(path);
    const LayerRowNode* row = menuRowForTest(path);
    if (!row) {
        return false;
    }
    QMenu menu;
    populateRowMenu(menu, path, row->color, *row);
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator() && action->text() == text) {
            return action->isEnabled();
        }
    }
    return false;
}


} // namespace pictura
