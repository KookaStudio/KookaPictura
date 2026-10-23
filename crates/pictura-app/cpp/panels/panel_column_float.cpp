#include "panel_column.h"

#include "panel_column_internal.h"

#include "frame.h"
#include "icons.h"
#include "panel_group.h"
#include "theme.h"

#include <QtCore/QRect>
#include <QtCore/QSize>
#include <QtGui/QGuiApplication>
#include <QtGui/QIcon>
#include <QtGui/QScreen>
#include <QtWidgets/QApplication>
#include <QtWidgets/QBoxLayout>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QScrollBar>
#include <QtWidgets/QSplitter>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

PanelFloat* PanelColumn::createFloat(PanelGroup* group, const QPoint& globalPos)
{
    if (!group) {
        return nullptr;
    }
    PanelGroup* hosted = group;
    if (dragIsPanel_ && !dragPanel_.isEmpty()) {
        // M43: a tab drag floats a one-panel group holding only the dragged
        // panel; the source stack (docked group or old float) keeps the rest.
        QString title;
        QIcon iconValue;
        int index = -1;
        QWidget* panel = group->takePanel(dragPanel_, &title, &iconValue, &index);
        if (!panel) {
            return nullptr;
        }
        hosted = new PanelGroup(this);
        hosted->addPanel(panel, title, iconValue);
        panelVisible_[dragPanel_] = true;
        wireGroup(hosted);
        // M44 W4: keep the (possibly now empty) source group alive so its tab
        // bar keeps the implicit mouse grab until release; commitDrop/cancelDrag
        // clean it up once the drag ends.
    }
    const int index = groups_.indexOf(hosted);
    if (!dragIsPanel_) {
        // A whole group dragged out of the flyout takes the flyout with it.
        detachFlyoutForDrag(group);
    }
    if (index >= 0) {
        groups_.removeAt(index);
        hosted->setParent(nullptr);
    }
    // The overlay parents to the main window (transient for it) so it stays
    // above the frame; it must not parent to the `centerSplitter`, which would
    // absorb it as a splitter pane, nor to a float, which would nest it.
    QWidget* host = owningFrame();
    if (!host) {
        host = window();
    }
    if (!host) {
        host = this;
    }
    auto* floatWindow = new PanelFloat(host);
    floatWindow->setGroup(hosted);
    floatWindow->onClose = [this, floatWindow]() { closeFloat(floatWindow); };
    hosted->setVisible(true);
    QSize size = hosted->sizeHint();
    size = size.expandedTo(QSize(220, 120));
    if (size.width() > 520) {
        size.setWidth(520);
    }
    floatWindow->resize(size);
    moveFloat(floatWindow, globalPos - dragGrabOffset_);
    floatWindow->show();
    // M47: an iconic hosted group needs room for its icon row.
    if (hosted->isCollapsedToIcons()
        && floatWindow->height() < PanelFloat::kFloatIconMinHeight) {
        floatWindow->resize(floatWindow->width(), PanelFloat::kFloatIconMinHeight);
    }
    floatWindow->syncToContent();
    floatWindow->raise();
    floats_ << floatWindow;
    // ponytail: the strip row stays stale during the drag (rebuilt on
    // commit/cancel), because rebuilding it deletes the widget holding the
    // mouse grab.
    // Re-dock routes through the float's group, so commitDrop takes the panel
    // out of the one-panel float (or re-inserts the whole group).
    dragGroup_ = hosted;
    // A tab drag that leaves the flyout's group with no visible panel empties
    // the popup; close it so it cannot linger as an empty rectangle. A drag
    // that leaves visible panels keeps the popup open on them.
    if (flyout_ && flyout_->isVisible() && flyoutGroup_ == group
        && (hosted == group || group->visibleTitles().isEmpty())) {
        closeIconFlyout();
    }
    return floatWindow;
}

QRect PanelColumn::floatBounds(QWidget* host) const
{
    // In child mode the overlay is clipped to the owning frame, so it is bounded
    // by the frame rect (in global coordinates); only in top-level mode may it
    // sit anywhere on the screen, bounded by the screen's available geometry so
    // it cannot be lost.
    if (!PanelFloat::overlayUsesTopLevel() && host) {
        return QRect(host->mapToGlobal(QPoint(0, 0)), host->size());
    }
    QScreen* screen = host ? host->screen() : nullptr;
    if (!screen) {
        screen = QGuiApplication::primaryScreen();
    }
    return screen ? screen->availableGeometry() : QRect();
}

void PanelColumn::moveFloat(PanelFloat* floatWindow, const QPoint& globalTopLeft)
{
    if (!floatWindow) {
        return;
    }
    if (!PanelFloat::overlayUsesTopLevel()) {
        // A child moves in parent-relative coordinates and is clamped to the
        // frame rect; `QWidget::move` would ignore global coordinates here.
        QWidget* host = floatWindow->parentWidget();
        if (host) {
            const QPoint local = host->mapFromGlobal(globalTopLeft);
            const int maxX = qMax(0, host->width() - floatWindow->width());
            const int maxY = qMax(0, host->height() - floatWindow->height());
            floatWindow->move(qBound(0, local.x(), maxX), qBound(0, local.y(), maxY));
            return;
        }
    }
    QScreen* screen = QGuiApplication::screenAt(globalTopLeft);
    if (!screen) {
        screen = floatWindow->parentWidget() ? floatWindow->parentWidget()->screen() : nullptr;
    }
    if (!screen) {
        screen = QGuiApplication::primaryScreen();
    }
    const QRect bounds = screen ? screen->availableGeometry() : QRect();
    const int maxX = qMax(bounds.left(), bounds.right() - floatWindow->width() + 1);
    const int maxY = qMax(bounds.top(), bounds.bottom() - floatWindow->height() + 1);
    floatWindow->move(qBound(bounds.left(), globalTopLeft.x(), maxX),
                      qBound(bounds.top(), globalTopLeft.y(), maxY));
}

void PanelColumn::destroyFloat(PanelFloat* floatWindow)
{
    if (!floatWindow) {
        return;
    }
    if (!floats_.contains(floatWindow)) {
        // Already torn down by a cleanup path in this drop; do not double-free.
        if (dragFloat_ == floatWindow) {
            dragFloat_ = nullptr;
        }
        maybeRemoveSelf();
        return;
    }
    floats_.removeAll(floatWindow);
    if (dragFloat_ == floatWindow) {
        dragFloat_ = nullptr;
    }
    if (floatIndicator_ == floatWindow) {
        floatIndicator_ = nullptr;
    }
    floatWindow->hide();
    floatWindow->deleteLater();
    maybeRemoveSelf();
}

void PanelColumn::closeFloat(PanelFloat* floatWindow)
{
    if (!floatWindow) {
        return;
    }
    PanelGroup* group = floatWindow->group();
    if (group && group->parentWidget() == floatWindow) {
        group->setParent(nullptr);
        insertGroupAt(group, groups_.size());
        // Destroy the shell before `closeGroup` can trigger `maybeRemoveSelf`,
        // which would re-home this float and leave the empty overlay behind.
        destroyFloat(floatWindow);
        closeGroup(group);
        return;
    }
    destroyFloat(floatWindow);
}

void PanelColumn::rehomeFloatsTo(PanelColumn* target)
{
    if (!target || target == this) {
        return;
    }
    for (PanelFloat* floatWindow : floats_) {
        if (!floatWindow) {
            continue;
        }
        PanelGroup* group = floatWindow->group();
        if (!group) {
            continue;
        }
        QObject::disconnect(group, nullptr, this, nullptr);
        wired_.remove(group);
        target->wireGroup(group);
        // The float's close callback captured this (now-removed) column; repoint
        // it at the target so the re-homed overlay's close button stays valid.
        floatWindow->onClose = [target, floatWindow]() { target->closeFloat(floatWindow); };
        for (QWidget* panel : group->panels()) {
            if (panel) {
                target->panelVisible_[panel->objectName()] =
                    group->isPanelVisible(panel->objectName());
            }
        }
    }
    target->floats_.append(floats_);
    floats_.clear();
}

PanelFloat* PanelColumn::restoreGroupFloat(PanelGroup* group, const QRect& geometry)
{
    if (!group || !geometry.isValid()) {
        return nullptr;
    }
    const int index = groups_.indexOf(group);
    if (index >= 0) {
        groups_.removeAt(index);
    }
    group->setParent(nullptr);
    QWidget* host = owningFrame();
    if (!host) {
        host = window();
    }
    if (!host) {
        if (index >= 0) {
            insertGroupAt(group, index);
        }
        return nullptr;
    }
    auto* floatWindow = new PanelFloat(host);
    floatWindow->setGroup(group);
    floatWindow->onClose = [this, floatWindow]() { closeFloat(floatWindow); };
    group->setFlyout(false);
    group->setVisible(true);
    QSize size = geometry.size()
                     .expandedTo(QSize(PanelFloat::kFloatMinWidth, PanelFloat::kFloatMinHeight));
    const QRect bounds = floatBounds(host);
    if (bounds.isValid()) {
        size = size.boundedTo(bounds.size())
                   .expandedTo(QSize(PanelFloat::kFloatMinWidth, PanelFloat::kFloatMinHeight));
    }
    floatWindow->resize(size);
    moveFloat(floatWindow, geometry.topLeft());
    floatWindow->show();
    floatWindow->raise();
    floats_ << floatWindow;
    maybeRemoveSelf();
    return floatWindow;
}

PanelFloat* PanelColumn::restoreColumnFloat(const QRect& geometry)
{
    if (!geometry.isValid()) {
        return columnFloat_;
    }
    PanelFloat* floatWindow = floatColumn(geometry.topLeft());
    if (!floatWindow) {
        return nullptr;
    }
    if (!toolsContent_) {
        QSize size = geometry.size()
                         .expandedTo(QSize(PanelFloat::kFloatMinWidth, PanelFloat::kFloatMinHeight));
        const QRect bounds = floatBounds(floatWindow->parentWidget());
        if (bounds.isValid()) {
            size = size.boundedTo(bounds.size())
                       .expandedTo(QSize(PanelFloat::kFloatMinWidth, PanelFloat::kFloatMinHeight));
        }
        floatWindow->resize(size);
    }
    moveFloat(floatWindow, geometry.topLeft());
    floatWindow->show();
    floatWindow->raise();
    return floatWindow;
}

QRect PanelColumn::columnFloatGeometry() const
{
    if (!columnFloat_) {
        return QRect();
    }
    return QRect(columnFloat_->mapToGlobal(QPoint(0, 0)), columnFloat_->size());
}

void PanelColumn::reclampFloats()
{
    auto clamp = [this](PanelFloat* floatWindow) {
        if (!floatWindow) {
            return;
        }
        const QRect bounds = floatBounds(floatWindow->parentWidget());
        QSize size = floatWindow->size();
        if (bounds.isValid()) {
            size = size.boundedTo(bounds.size()).expandedTo(floatWindow->minimumSize());
        }
        floatWindow->resize(size);
        moveFloat(floatWindow, floatWindow->mapToGlobal(QPoint(0, 0)));
    };
    for (PanelFloat* floatWindow : floats_) {
        clamp(floatWindow);
    }
    clamp(columnFloat_);
}

void PanelColumn::redockGroupFloats()
{
    const QList<PanelFloat*> floats = floats_;
    for (PanelFloat* floatWindow : floats) {
        if (!floatWindow) {
            continue;
        }
        PanelGroup* group = floatWindow->group();
        if (!group) {
            continue;
        }
        floats_.removeAll(floatWindow);
        floatWindow->hide();
        group->setParent(nullptr);
        group->setCollapsedToIcons(false);
        insertGroupAt(group, groups_.size());
        floatWindow->deleteLater();
    }
}

PanelFloat* PanelColumn::groupFloatAtGlobal(const QPoint& globalPos) const
{
    for (PanelFloat* floatWindow : floats_) {
        if (!floatWindow || !floatWindow->group()) {
            continue;
        }
        if (QRect(floatWindow->mapToGlobal(QPoint(0, 0)), floatWindow->size()).contains(globalPos)) {
            return floatWindow;
        }
    }
    return nullptr;
}

PanelFloat* PanelColumn::floatColumn(const QPoint& globalTopLeft)
{
    if (columnFloat_) {
        return columnFloat_;
    }
    QWidget* host = owningFrame();
    if (!host) {
        host = window();
    }
    if (!host) {
        return nullptr;
    }
    // Remember the docked side before the column leaves the splitter so a
    // restored/floating Tools or left column keeps its flyout/redock side.
    lastSide_ = side();
    auto* floatWindow = new PanelFloat(host);
    floatWindow->setContent(this);
    if (toolsContent_) {
        // The floating tools column is not resizable and hugs its content: the
        // tool grid width and the minimum height the content needs. The docked
        // scroll area otherwise pins the toolbox to the viewport height, so the
        // overlay lets the toolbox take its own hint while it floats.
        if (toolsScroll_) {
            toolsScroll_->setWidgetResizable(false);
        }
        floatWindow->setMinimumWidth(0);
        floatWindow->setResizable(false);
    } else {
        // Keep the on-screen width (never below this column's minimum) so a
        // re-dock lands in the slot the column left; the overlay carries the
        // shared minimum width and the size grip for both group and column
        // payloads. The default height is about two thirds of the docked height
        // so a floated column leaves room on the desktop.
        const int width =
            qMax(minimumWidth(), this->width() > 0 ? this->width() : kDefaultNormalWidth);
        const int docked = this->height() > 0 ? this->height() : 240;
        const int height = qMax(PanelFloat::kFloatMinHeight, (docked * 2) / 3);
        floatWindow->resize(width, height);
    }
    moveFloat(floatWindow, globalTopLeft);
    floatWindow->show();
    floatWindow->raise();
    // An already-iconic widget column keeps the two-thirds initial height; the
    // snap happens only when its rail mode is toggled while floating. The tools
    // column still hugs its content.
    if (toolsContent_ || !railMode_) {
        floatWindow->syncToContent();
    }
    columnFloat_ = floatWindow;
    return floatWindow;
}

bool PanelColumn::redockColumnFloat(int side, PanelColumn* anchor)
{
    auto* frame = owningFrame();
    if (!columnFloat_ || !frame) {
        return false;
    }
    // The frame's move path reparents the column out of the overlay and into
    // the splitter; only then is the empty overlay safe to delete.
    if (!frame->movePanelColumn(this, side, anchor)) {
        return false;
    }
    destroyColumnFloat();
    return true;
}

void PanelColumn::destroyColumnFloat()
{
    if (!columnFloat_) {
        return;
    }
    PanelFloat* floatWindow = columnFloat_;
    columnFloat_ = nullptr;
    if (toolsScroll_) {
        toolsScroll_->setWidgetResizable(true);
    }
    floatWindow->hide();
    floatWindow->deleteLater();
}

void PanelColumn::cancelColumnFloat()
{
    if (!columnFloat_) {
        return;
    }
    // Detach this column from the overlay before the overlay is destroyed. The
    // column is the overlay's child, so deleting the overlay without reparenting
    // would take the column with it; this keeps the call self-contained.
    setParent(nullptr);
    destroyColumnFloat();
}

void PanelColumn::updateColumnDrag(const QPoint& globalPos)
{
    auto* frame = owningFrame();
    if (!frame) {
        return;
    }
    int side = -1;
    PanelColumn* anchor = frame->resolveColumnMoveTarget(globalPos, this, &side);
    const bool hasTarget = side >= 0;
    // M48: the whole column tears off into the floating overlay on the first
    // move and follows the cursor for the rest of the drag, whether or not a
    // target resolves; a resolved target still draws the edge indicator and
    // re-docks on release. This mirrors the group drag's float; no second drag
    // system.
    if (!columnFloat_) {
        floatColumn(globalPos - columnGrabOffset_);
    } else {
        moveFloat(columnFloat_, globalPos - columnGrabOffset_);
    }
    // Phase 6: a whole-column drag dims its overlay for the whole drag.
    setDragDimTarget(columnFloat_);
    // A bare workspace edge resolves no anchor. Mark the central area's own
    // left/right edge (where the new column actually lands), not an arbitrary
    // column's edge: the outermost column only owns the mark so the drawn line
    // and the resolved target stay in one column. Without this a left-side drop
    // drew its line on a right-hand column.
    PanelColumn* indicator = anchor;
    bool workspaceEdge = false;
    if (!indicator && hasTarget) {
        const QList<PanelColumn*> columns = frame->panelColumns();
        PanelColumn* outer = nullptr;
        if (side == 0) {
            for (PanelColumn* column : columns) {
                if (column && column != this && column->isVisible()) {
                    outer = column;
                    break;
                }
            }
        } else {
            for (int i = columns.size() - 1; i >= 0; --i) {
                PanelColumn* column = columns.at(i);
                if (column && column != this && column->isVisible()) {
                    outer = column;
                    break;
                }
            }
        }
        indicator = outer ? outer : this;
        workspaceEdge = true;
    }
    if (columnDropAnchor_ && columnDropAnchor_ != indicator) {
        columnDropAnchor_->hideEdgeDropIndicator();
    }
    columnDropAnchor_ = indicator;
    columnDropSide_ = side;
    if (indicator && hasTarget) {
        if (workspaceEdge) {
            indicator->showWorkspaceEdgeIndicator(side == 0);
        } else {
            indicator->showEdgeDropIndicator(side == 0 ? PanelSide::Left : PanelSide::Right);
        }
    }
}

bool PanelColumn::finishColumnDrag(const QPoint& globalPos)
{
    setDragDimTarget(nullptr);
    auto* frame = owningFrame();
    if (!frame) {
        return false;
    }
    int side = -1;
    PanelColumn* anchor = frame->resolveColumnMoveTarget(globalPos, this, &side);
    if (columnDropAnchor_) {
        columnDropAnchor_->hideEdgeDropIndicator();
    }
    columnDropAnchor_ = nullptr;
    columnDropSide_ = -1;
    if (side >= 0) {
        // A valid target re-places the column; a floating one redocks into the
        // splitter at the resolved boundary.
        return columnFloat_ ? redockColumnFloat(side, anchor)
                            : frame->movePanelColumn(this, side, anchor);
    }
    // No target: the column stays torn off in the floating overlay instead of
    // doing nothing, and a drag that never moved floats it now.
    if (!columnFloat_) {
        return floatColumn(globalPos - columnGrabOffset_) != nullptr;
    }
    return true;
}

} // namespace pictura
