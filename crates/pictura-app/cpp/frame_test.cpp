#include "frame_includes.h"

namespace pictura {

QString PicturaMainWindow::panelColumnSideForTest(int index) const
{    const QList<PanelColumn*> columns = panelColumns();    if (index < 0 || index >= columns.size()) {
        return QString();
    }
    return sideOf(columns.at(index)) == PanelSide::Left ? QStringLiteral("left")
                                                        : QStringLiteral("right");
}

bool PicturaMainWindow::newColumnDropForTest(const QString& panelName, const QString& side)
{
    PanelColumn* source = columnForPanel(panelName);
    if (!source || !centralWidget()) {
        return false;
    }
    const int before = columnCount();
    const QRect central(centralWidget()->mapToGlobal(QPoint(0, 0)), centralWidget()->size());
    QPoint point;
    int expected = -1;
    if (side == QStringLiteral("left")) {
        point = QPoint(central.left() + 2, central.center().y());
        expected = 0;
    } else if (side == QStringLiteral("right")) {
        point = QPoint(central.right() - 2, central.center().y());
        expected = 1;
    } else if (side == QStringLiteral("tools")) {
        // D1: the Tools panel is now a column; a widget-panel drop at its outer
        // (left) edge allocates a new sibling column on that side.
        if (!toolsColumn_ || !toolsColumn_->isVisible()) {
            return false;
        }
        const QRect tr(toolsColumn_->mapToGlobal(QPoint(0, 0)), toolsColumn_->size());
        point = QPoint(tr.left() - 8, tr.center().y());
        expected = 0;
    } else {
        return false;
    }
    if (!source->beginTabDragForTest(panelName)) {
        return false;
    }
    source->dragToForTest(point);
    const bool dropped = source->dropForTest(point);
    QCoreApplication::processEvents();
    PanelColumn* destination = columnForPanel(panelName);
    if (!dropped || !destination || destination == source) {
        return false;
    }
    if (columnCount() != before + 1) {
        return false;
    }
    const PanelSide want = expected == 0 ? PanelSide::Left : PanelSide::Right;
    return sideOf(destination) == want;
}

bool PicturaMainWindow::newColumnBesideForTest(const QString& panelName,
                                               const QString& anchorPanel)
{
    PanelColumn* source = columnForPanel(panelName);
    PanelColumn* anchor = columnForPanel(anchorPanel);
    if (!source || !anchor || source == anchor) {
        return false;
    }
    const int before = columnCount();
    const QRect r(anchor->mapToGlobal(QPoint(0, 0)), anchor->size());
    // Just outside the anchor's left edge: the resolver treats this as a new
    // column anchored immediately before `anchor`.
    const QPoint point(r.left() - 8, r.center().y());
    if (!source->beginTabDragForTest(panelName)) {
        return false;
    }
    source->dragToForTest(point);
    const bool dropped = source->dropForTest(point);
    QCoreApplication::processEvents();
    PanelColumn* destination = columnForPanel(panelName);
    if (!dropped || !destination || destination == source || columnCount() != before + 1) {
        return false;
    }
    const int destIndex = centerSplitter_->indexOf(destination);
    const int anchorIndex = centerSplitter_->indexOf(anchor);
    return destIndex >= 0 && anchorIndex >= 0 && qAbs(destIndex - anchorIndex) == 1;
}

bool PicturaMainWindow::dropIntoGroupForTest(const QString& panelName, const QString& targetPanel,
                                             int index)
{
    PanelColumn* source = columnForPanel(panelName);
    PanelColumn* destination = columnForPanel(targetPanel);
    if (!source || !destination) {
        return false;
    }
    PanelGroup* targetGroup = destination->groupForPanel(targetPanel);
    if (!targetGroup) {
        return false;
    }
    QPoint point;
    if (destination->railMode()) {
        point = destination->stripEntryPointForTest(targetPanel, 0);
    } else {
        destination->ensureGroupVisibleForTest(targetPanel);
        for (int i = 0; i < 4; ++i) {
            QCoreApplication::processEvents();
        }
        const int at = index < 0 ? targetGroup->titleCountForTest()
                                 : qMin(index, targetGroup->titleCountForTest());
        point = targetGroup->tabInsertionGlobalPointForTest(at);
    }
    if (point.isNull() || !source->beginTabDragForTest(panelName)) {
        return false;
    }
    source->dragToForTest(point);
    const bool dropped = source->dropForTest(point);
    for (int i = 0; i < 4; ++i) {
        QCoreApplication::processEvents();
    }
    PanelColumn* owner = columnForPanel(panelName);
    return dropped && owner && owner->groupForPanel(panelName) == targetGroup;
}

bool PicturaMainWindow::dropBoundaryForTest(const QString& panelName, const QString& targetPanel,
                                            bool above)
{
    PanelColumn* source = columnForPanel(panelName);
    PanelColumn* destination = columnForPanel(targetPanel);
    if (!source || !destination) {
        return false;
    }
    PanelGroup* targetGroup = destination->groupForPanel(targetPanel);
    if (!targetGroup) {
        return false;
    }
    QPoint point;
    if (destination->railMode()) {
        // M44 C3: compact boundaries are the inter-group dividers / strip ends,
        // not a button edge (a button edge now inserts into its group).
        const int gi = destination->compactStripGroupIndexForTest(targetPanel);
        if (gi < 0) {
            return false;
        }
        point = destination->compactStripBoundaryPointForTest(above ? gi : gi + 1);
    } else {
        destination->ensureGroupVisibleForTest(targetPanel);
        for (int i = 0; i < 4; ++i) {
            QCoreApplication::processEvents();
        }
        const QRect groupRect(targetGroup->mapToGlobal(QPoint(0, 0)), targetGroup->size());
        // Stay clear of the tab bar so the drop resolves the group's top/bottom
        // half, not a tab insertion.
        const int barBottom = targetGroup->tabBarGlobalRect().bottom();
        const int top = qMax(groupRect.top(), barBottom + 1);
        const int height = qMax(1, groupRect.bottom() - top);
        point = QPoint(groupRect.center().x(),
                       above ? top + height / 4 : top + (height * 3) / 4);
    }
    PanelGroup* original = source->groupForPanel(panelName);
    if (point.isNull() || !source->beginTabDragForTest(panelName)) {
        return false;
    }
    source->dragToForTest(point);
    const bool dropped = source->dropForTest(point);
    for (int i = 0; i < 4; ++i) {
        QCoreApplication::processEvents();
    }
    // A panel boundary drop leaves the panel alone in a fresh group.
    PanelColumn* owner = columnForPanel(panelName);
    PanelGroup* landed = owner ? owner->groupForPanel(panelName) : nullptr;
    return dropped && landed && landed != original && landed->titleCountForTest() == 1;
}

void PicturaMainWindow::setDocumentNameForTest(int index, const QString& name)
{
    if (index < 0 || index >= docs_.size()) {
        return;
    }
    docs_[index]->displayName = name;
    updateTabTitle(index);
}

bool PicturaMainWindow::reorderDocumentsForTest()
{
    if (!tabs_ || !tabs_->tabBar()) {
        return false;
    }
    newDocument(QStringLiteral("ReorderA"), 4, 3, QStringLiteral("rgb"), 8,
                QStringLiteral("white"));
    newDocument(QStringLiteral("ReorderB"), 4, 3, QStringLiteral("rgb"), 8,
                QStringLiteral("white"));
    const int docA = docs_.size() - 2;
    const int docB = docs_.size() - 1;
    if (docA < 0) {
        return false;
    }
    PictureView* a = viewAt(docA);
    PictureView* b = viewAt(docB);
    const int from = tabOf(docs_.at(docA)->windows.value(0));
    const int to = tabOf(docs_.at(docB)->windows.value(0));
    if (from < 0 || to < 0) {
        return false;
    }
    const QString nameA = documentName(docA);
    const bool activeBefore = viewAt(activeDocumentIndex()) == b;
    tabs_->tabBar()->moveTab(from, to);
    QCoreApplication::processEvents();
    const DocWindow* atFrom = windowAtTab(from);
    const DocWindow* atTo = windowAtTab(to);
    return activeBefore && a && b && atFrom && atTo && atFrom->doc->view == b
           && atTo->doc->view == a && windows_.at(from) == atFrom && windows_.at(to) == atTo
           && viewAt(docA) == a && documentName(docA) == nameA
           && viewAt(activeDocumentIndex()) == b;
}

} // namespace pictura
