#include "panel_column.h"

#include "panel_column_internal.h"

#include "frame.h"
#include "icons.h"
#include "panel_group.h"
#include "theme.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QEvent>
#include <QtCore/QJsonObject>
#include <QtCore/QMetaObject>
#include <QtCore/QRect>
#include <QtCore/QSize>
#include <QtGui/QAction>
#include <QtGui/QCursor>
#include <QtGui/QFontMetrics>
#include <QtGui/QGuiApplication>
#include <QtGui/QHideEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPalette>
#include <QtGui/QScreen>
#include <QtGui/QShowEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QBoxLayout>
#include <QtWidgets/QFrame>
#include <QtWidgets/QGraphicsOpacityEffect>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMainWindow>
#include <QtWidgets/QMenu>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QScrollBar>
#include <QtWidgets/QSplitter>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

QString PanelColumn::groupOfForTest(const QString& objectName) const
{
    PanelGroup* group = groupForPanel(objectName);
    return group ? group->objectName() : QString();
}

bool PanelColumn::scrollableForTest() const
{
    return scroll_ != nullptr;
}

int PanelColumn::toolsScrollBarPolicyForTest() const
{
    return toolsScroll_ ? static_cast<int>(toolsScroll_->verticalScrollBarPolicy()) : -1;
}

int PanelColumn::minimumWidthForTest() const
{
    return minimumWidth();
}

bool PanelColumn::iconStripVisibleForTest() const
{
    return iconStrip_ && iconStrip_->isVisible();
}

int PanelColumn::dividerCountForTest() const
{
    return iconStrip_
        ? static_cast<int>(
              iconStrip_->findChildren<QFrame*>(QStringLiteral("panelIconDivider")).size())
        : 0;
}

int PanelColumn::groupDividerWidthForTest() const
{
    return splitter_ ? splitter_->handleWidth() : 0;
}

QString PanelColumn::stripLabelTextForTest(const QString& objectName) const
{
    for (int i = 0; i < stripEntries_.size() && i < stripLabels_.size(); ++i) {
        if (stripEntries_.at(i).name == objectName && stripLabels_.at(i)) {
            return stripLabels_.at(i)->text();
        }
    }
    return QString();
}

bool PanelColumn::stripLabelVisibleForTest(const QString& objectName) const
{
    for (int i = 0; i < stripEntries_.size() && i < stripLabels_.size(); ++i) {
        if (stripEntries_.at(i).name == objectName && stripLabels_.at(i)) {
            return stripLabels_.at(i)->isVisible();
        }
    }
    return false;
}

void PanelColumn::setIconStripWidthForTest(int width)
{
    if (!iconStrip_) {
        return;
    }
    // Rail mode pins the column to a fixed narrow width, so the strip alone
    // cannot reach a wider test width; widen the column too so the requested
    // strip width is real and the label behaviour can be exercised.
    const int target = qMax(0, width);
    setFixedWidth(target);
    iconStrip_->resize(target, iconStrip_->height());
    updateIconStripLabels();
}

bool PanelColumn::openIconFlyoutForTest(const QString& objectName)
{
    if (!resolveFlyoutGroup(objectName)) {
        return false;
    }
    QToolButton* button =
        iconStrip_ ? iconStrip_->findChild<QToolButton*>(QStringLiteral("panelIcon_") + objectName)
                   : nullptr;
    if (!button) {
        for (PanelGroup* group : groups_) {
            button = group->findChild<QToolButton*>(QStringLiteral("panelGroupIcon_") + objectName);
            if (button) {
                break;
            }
        }
    }
    if (!button) {
        for (PanelFloat* floatWindow : floats_) {
            PanelGroup* group = floatWindow ? floatWindow->group() : nullptr;
            if (!group) {
                continue;
            }
            button = group->findChild<QToolButton*>(
                QStringLiteral("panelGroupIcon_") + objectName);
            if (button) {
                break;
            }
        }
    }
    const QPoint pos =
        button ? button->mapToGlobal(QPoint(button->width(), 0)) : QCursor::pos();
    openIconFlyout(objectName, pos);
    return true;
}

bool PanelColumn::iconFlyoutVisibleForTest() const
{
    return flyout_ && flyout_->isVisible();
}

QStringList PanelColumn::tabMenuActionsForTest() const
{
    const PanelGroup* group = menuGroup_;
    if (!group && !groups_.isEmpty()) {
        group = groups_.first();
    }
    return tabMenuTextsFor(group);
}

QStringList PanelColumn::tabMenuActionsForTest(const QString& groupObjectName) const
{
    return tabMenuTextsFor(findGroupByName(groupObjectName));
}

bool PanelColumn::triggerTabMenuForTest(const QString& text)
{
    PanelGroup* group = menuGroup_;
    if (!group && !groups_.isEmpty()) {
        group = groups_.first();
    }
    if (!group) {
        return false;
    }
    menuGroup_ = group;
    QMenu* menu = buildTabMenu(group);
    bool found = false;
    for (QAction* action : menu->actions()) {
        if (action->text() == text) {
            action->trigger();
            found = true;
            break;
        }
    }
    delete menu;
    return found;
}

bool PanelColumn::beginTabDragForTest(const QString& objectName)
{
    PanelGroup* group = groupForPanel(objectName);
    if (!group) {
        for (PanelFloat* floatWindow : floats_) {
            PanelGroup* candidate = floatWindow ? floatWindow->group() : nullptr;
            if (candidate && candidate->containsPanel(objectName)) {
                group = candidate;
                break;
            }
        }
    }
    if (!group) {
        return false;
    }
    beginPanelDrag(group, objectName, group->mapToGlobal(QPoint(0, 0)));
    return true;
}

bool PanelColumn::beginGroupDragForTest(const QString& panelName)
{
    PanelGroup* group = groupForPanel(panelName);
    if (!group) {
        for (PanelFloat* floatWindow : floats_) {
            PanelGroup* candidate = floatWindow ? floatWindow->group() : nullptr;
            if (candidate && candidate->containsPanel(panelName)) {
                group = candidate;
                break;
            }
        }
    }
    if (!group) {
        return false;
    }
    beginGroupDrag(group, group->mapToGlobal(QPoint(0, 0)));
    return true;
}

void PanelColumn::dragToForTest(const QPoint& globalPos)
{
    updateDrag(globalPos);
}

bool PanelColumn::dropForTest(const QPoint& globalPos)
{
    if (!dragActive_) {
        return false;
    }
    updateDrag(globalPos);
    return commitDrop();
}

void PanelColumn::cancelDragForTest()
{
    cancelDrag();
}

bool PanelColumn::selfAnchorDockForTest(const QString& objectName, bool toRight)
{
    auto* frame = owningFrame();
    if (!frame) {
        return false;
    }
    PanelColumn* source = frame->columnForPanel(objectName);
    if (!source) {
        return false;
    }
    const int before = frame->panelColumns().size();
    const QRect r(source->mapToGlobal(QPoint(0, 0)), source->size());
    const QPoint point(toRight ? r.right() + 8 : r.left() - 8, r.center().y());
    if (!source->beginTabDragForTest(objectName)) {
        return false;
    }
    source->dragToForTest(point);
    const bool dropped = source->dropForTest(point);
    QCoreApplication::processEvents();
    PanelColumn* destination = frame->columnForPanel(objectName);
    if (!dropped || !destination || destination == source
        || frame->panelColumns().size() != before + 1) {
        return false;
    }
    // The source column survives (the test uses the multi-group primary), so
    // the new column must sit immediately on the requested side of it.
    const int destinationIndex = frame->panelColumns().indexOf(destination);
    const int sourceIndex = frame->panelColumns().indexOf(source);
    if (destinationIndex < 0 || sourceIndex < 0) {
        return false;
    }
    return toRight ? destinationIndex == sourceIndex + 1 : destinationIndex == sourceIndex - 1;
}

qreal PanelColumn::dragDimOpacityForTest() const
{
    if (auto* floatWindow = qobject_cast<PanelFloat*>(dimTarget_)) {
        return floatWindow->dragOpacityForTest();
    }
    if (auto* effect = qobject_cast<QGraphicsOpacityEffect*>(
            dimTarget_ ? dimTarget_->graphicsEffect() : nullptr)) {
        return effect->opacity();
    }
    return 1.0;
}

bool PanelColumn::dropIndicatorVisibleForTest() const
{
    return (indicator_ && indicator_->isVisible())
           || (stripIndicator_ && stripIndicator_->isVisible())
           || (edgeIndicator_ && edgeIndicator_->isVisible());
}

QRect PanelColumn::dropIndicatorGeometryForTest() const
{
    if (edgeIndicator_ && edgeIndicator_->isVisible()) {
        return edgeIndicator_->geometry();
    }
    if (stripIndicator_ && stripIndicator_->isVisible()) {
        return stripIndicator_->geometry();
    }
    return indicator_ ? indicator_->geometry() : QRect();
}

bool PanelColumn::edgeIndicatorOnFrameForTest() const
{
    return edgeIndicator_ && edgeIndicator_->parentWidget()
           && edgeIndicator_->parentWidget() == owningFrame();
}

int PanelColumn::scrollViewportHeightForTest() const
{
    return scroll_ ? scroll_->viewport()->height() : 0;
}

QRect PanelColumn::dropIndicatorGlobalGeometryForTest() const
{
    if (edgeIndicator_ && edgeIndicator_->isVisible()) {
        return QRect(edgeIndicator_->mapToGlobal(QPoint(0, 0)), edgeIndicator_->size());
    }
    if (stripIndicator_ && stripIndicator_->isVisible()) {
        return QRect(stripIndicator_->mapToGlobal(QPoint(0, 0)), stripIndicator_->size());
    }
    if (indicator_ && indicator_->isVisible()) {
        return QRect(indicator_->mapToGlobal(QPoint(0, 0)), indicator_->size());
    }
    return QRect();
}

bool PanelColumn::outlineIndicatorVisibleForTest() const
{
    return outlineIndicator_ && outlineIndicator_->isVisible();
}

int PanelColumn::horizontalScrollPolicyForTest() const
{
    return scroll_ ? static_cast<int>(scroll_->horizontalScrollBarPolicy()) : -1;
}

int PanelColumn::horizontalScrollRangeForTest() const
{
    return scroll_ ? scroll_->horizontalScrollBar()->maximum() : 0;
}

int PanelColumn::minimumWidthFloorForTest() const
{
    return gSharedFloor;
}

int PanelColumn::contentMinimumWidthForTest() const
{
    return (scroll_ && scroll_->widget()) ? scroll_->widget()->minimumSizeHint().width() : 0;
}

int PanelColumn::viewportWidthForTest() const
{
    return scroll_ ? scroll_->viewport()->width() : 0;
}

int PanelColumn::dropIndexForTest() const
{
    return dropTarget_.onTabBar ? dropTarget_.tabIndex : dropTarget_.boundary;
}

QStringList PanelColumn::stripOrderForTest() const
{
    QStringList out;
    for (const StripEntry& entry : stripEntries_) {
        out << entry.name;
    }
    return out;
}

bool PanelColumn::beginStripDragForTest(const QString& objectName)
{
    PanelGroup* group = groupForPanel(objectName);
    if (!group) {
        return false;
    }
    QToolButton* button = stripButtonFor(objectName);
    const QPoint pos = button ? button->mapToGlobal(QPoint(0, 0))
                              : mapToGlobal(QPoint(qMax(1, width() / 2), 0));
    beginPanelDrag(group, objectName, pos);
    return true;
}

QPoint PanelColumn::stripInsertionPointForTest(int index) const
{
    if (!iconStrip_ || stripEntries_.isEmpty()) {
        return iconStrip_ ? iconStrip_->mapToGlobal(QPoint(iconStrip_->width() / 2, 0)) : QPoint();
    }
    const int clamped = qBound(0, index, stripEntries_.size());
    QToolButton* anchor = clamped < stripEntries_.size() ? stripEntries_.at(clamped).button
                                                         : stripEntries_.last().button;
    if (!anchor) {
        return QPoint();
    }
    const QPoint topLeft = anchor->mapToGlobal(QPoint(0, 0));
    const int y = clamped < stripEntries_.size() ? topLeft.y()
                                                 : topLeft.y() + anchor->height();
    return QPoint(topLeft.x() + anchor->width() / 2, y);
}

int PanelColumn::stripDropIndexForTest() const
{
    return dropTarget_.onStrip ? dropTarget_.stripIndex : -1;
}

QPoint PanelColumn::stripEntryPointForTest(const QString& panelName, int where) const
{
    for (const StripEntry& entry : stripEntries_) {
        if (entry.name != panelName || !entry.button) {
            continue;
        }
        const QRect row(entry.button->mapToGlobal(QPoint(0, 0)), entry.button->size());
        switch (where) {
        case 1:
            return QPoint(row.center().x(), row.top());
        case 2:
            return QPoint(row.center().x(), row.bottom());
        default:
            return row.center();
        }
    }
    return QPoint();
}

bool PanelColumn::dropStripOnGroupForTest(const QString& objectName, const QString& targetPanel)
{
    PanelGroup* dest = groupForPanel(targetPanel);
    if (!dest) {
        return false;
    }
    setRailMode(true);
    if (!beginStripDragForTest(objectName)) {
        setRailMode(false);
        return false;
    }
    // Reveal the normal-mode group stack underneath the active strip drag, then
    // resolve the drop through the existing group/tab-bar target path.
    setRailMode(false);
    ensureGroupVisibleForTest(targetPanel);
    QCoreApplication::processEvents();
    const QPoint target = dest->tabInsertionGlobalPointForTest(0);
    dragToForTest(target);
    const bool dropped = dropForTest(target);
    return dropped && groupForPanel(objectName) == dest;
}

QString PanelColumn::flyoutSideForTest() const
{
    return flyoutSide();
}

QRect PanelColumn::iconFlyoutGeometryForTest() const
{
    if (!flyout_ || !flyout_->isVisible()) {
        return QRect();
    }
    return QRect(flyout_->mapToGlobal(QPoint(0, 0)), flyout_->size());
}

QString PanelColumn::flyoutHeaderTitleForTest() const
{
    // M45 C1: the popup chrome is the hosted group's own tab bar, so the
    // "header" title is the clicked panel's tab title.
    return flyoutGroup_ ? flyoutGroup_->titleForPanel(flyoutName_) : QString();
}

bool PanelColumn::flyoutHeaderCloseForTest() const
{
    // M45 C1: the hosted group's tab bar (with its per-widget `▾` menu, which
    // carries Close) is the popup's close affordance.
    return flyoutGroup_ && flyoutGroup_->tabBar() != nullptr;
}

bool PanelColumn::triggerFlyoutCloseForTest()
{
    closeIconFlyout();
    QCoreApplication::processEvents();
    return flyout_ ? !flyout_->isVisible() : true;
}

int PanelColumn::floatCountForTest() const
{
    return floats_.size();
}

PanelFloat* PanelColumn::floatForTest(int index) const
{
    if (index < 0 || index >= floats_.size()) {
        return nullptr;
    }
    return floats_.at(index);
}

QStringList PanelColumn::floatPanelNamesForTest(int index) const
{
    QStringList out;
    if (index < 0 || index >= floats_.size()) {
        return out;
    }
    PanelFloat* floatWindow = floats_.at(index);
    PanelGroup* group = floatWindow ? floatWindow->group() : nullptr;
    if (group) {
        for (QWidget* panel : group->panels()) {
            if (panel) {
                out << panel->objectName();
            }
        }
    }
    return out;
}

bool PanelColumn::floatTabIndicatorVisibleForTest(int index) const
{
    if (index < 0 || index >= floats_.size()) {
        return false;
    }
    PanelFloat* floatWindow = floats_.at(index);
    return floatWindow && floatWindow->tabIndicatorVisibleForTest();
}

bool PanelColumn::floatIsToolWindowForTest(int index) const
{
    if (index < 0 || index >= floats_.size()) {
        return false;
    }
    PanelFloat* floatWindow = floats_.at(index);
    if (!floatWindow) {
        return false;
    }
    const Qt::WindowFlags flags = floatWindow->windowFlags();
    // A frameless tool window: top-level, no decorations, and transient for the
    // main window (its parent), so it stays above the frame with no taskbar entry.
    return floatWindow->isWindow() && flags.testFlag(Qt::Tool)
           && flags.testFlag(Qt::FramelessWindowHint)
           && qobject_cast<QMainWindow*>(floatWindow->parentWidget()) != nullptr;
}

QRect PanelColumn::floatGeometryForTest(int index) const
{
    if (index < 0 || index >= floats_.size()) {
        return QRect();
    }
    PanelFloat* floatWindow = floats_.at(index);
    return QRect(floatWindow->mapToGlobal(QPoint(0, 0)), floatWindow->size());
}

QRect PanelColumn::floatHostRectForTest() const
{
    // Top-level mode clamps the float to the screen, child mode to the owning
    // frame; `floatBounds` reports whichever applies.
    return floatBounds(window());
}

bool PanelColumn::floatClampedForTest(int index, const QPoint& globalTopLeft)
{
    if (index < 0 || index >= floats_.size()) {
        return false;
    }
    moveFloat(floats_.at(index), globalTopLeft);
    QCoreApplication::processEvents();
    return floatHostRectForTest().contains(floatGeometryForTest(index));
}

bool PanelColumn::tearOffForTest(const QString& groupName)
{
    PanelGroup* group = findGroupByName(groupName);
    if (!group) {
        group = groupForPanel(groupName);
    }
    if (!group) {
        return false;
    }
    beginGroupDrag(group, group->mapToGlobal(QPoint(0, 0)));
    // Above the column is unambiguously outside every column and outside the
    // left/right new-column bands, so the drag tears off rather than resolving a
    // new-column target (which a horizontal offset can be, at the workspace edge).
    const QPoint outside = mapToGlobal(QPoint(width() / 2, -40));
    dragToForTest(outside);
    return dropForTest(outside);
}

bool PanelColumn::tearOffPanelForTest(const QString& objectName)
{
    PanelGroup* group = groupForPanel(objectName);
    if (!group) {
        for (PanelFloat* floatWindow : floats_) {
            PanelGroup* candidate = floatWindow ? floatWindow->group() : nullptr;
            if (candidate && candidate->containsPanel(objectName)) {
                group = candidate;
                break;
            }
        }
    }
    if (!group) {
        return false;
    }
    beginPanelDrag(group, objectName, group->mapToGlobal(QPoint(0, 0)));
    const QPoint outside = mapToGlobal(QPoint(width() / 2, -40));
    dragToForTest(outside);
    return dropForTest(outside);
}

bool PanelColumn::redockForTest(int floatIndex, int boundaryIndex)
{
    if (floatIndex < 0 || floatIndex >= floats_.size()) {
        return false;
    }
    PanelFloat* floatWindow = floats_.at(floatIndex);
    PanelGroup* group = floatWindow ? floatWindow->group() : nullptr;
    if (!group) {
        return false;
    }
    beginGroupDrag(group, group->mapToGlobal(QPoint(4, 4)));
    const QPoint target = boundaryPointForTest(boundaryIndex);
    dragToForTest(target);
    const bool dropped = dropForTest(target);
    return dropped && !floatForGroup(group);
}

bool PanelColumn::closeFloatForTest(int index)
{
    if (index < 0 || index >= floats_.size()) {
        return false;
    }
    PanelFloat* floatWindow = floats_.at(index);
    PanelGroup* group = floatWindow ? floatWindow->group() : nullptr;
    QToolButton* close = group ? group->floatCloseButton() : nullptr;
    if (!close) {
        return false;
    }
    close->click();
    QCoreApplication::processEvents();
    return true;
}

QToolButton* PanelColumn::floatCloseButtonForTest(int index) const
{
    if (index < 0 || index >= floats_.size()) {
        return nullptr;
    }
    PanelFloat* floatWindow = floats_.at(index);
    PanelGroup* group = floatWindow ? floatWindow->group() : nullptr;
    return group ? group->floatCloseButton() : nullptr;
}

bool PanelColumn::ensureGroupVisibleForTest(const QString& panelName)
{
    PanelGroup* group = groupForPanel(panelName);
    if (!group || !scroll_) {
        return false;
    }
    scroll_->ensureWidgetVisible(group, 0, 12);
    return true;
}

QWidget* PanelColumn::columnHeaderForTest() const
{
    return header_;
}

bool PanelColumn::beginColumnHeaderDragForTest(const QPoint& globalPos)
{
    columnPressPending_ = false;
    columnDragging_ = true;
    columnDragStartedFloating_ = isColumnFloating();
    columnPressGlobal_ = globalPos;
    columnGrabOffset_ = globalPos - mapToGlobal(QPoint(0, 0));
    updateColumnDrag(globalPos);
    return true;
}

void PanelColumn::dragColumnHeaderToForTest(const QPoint& globalPos)
{
    if (columnDragging_) {
        updateColumnDrag(globalPos);
    }
}

bool PanelColumn::dropColumnHeaderForTest(const QPoint& globalPos)
{
    if (!columnDragging_) {
        return false;
    }
    columnDragging_ = false;
    return finishColumnDrag(globalPos);
}

QStringList PanelColumn::columnHeaderMenuTextsForTest() const
{
    QStringList out;
    QMenu* menu = const_cast<PanelColumn*>(this)->buildColumnHeaderMenu();
    for (QAction* action : menu->actions()) {
        if (!action->isSeparator()) {
            out << action->text();
        }
    }
    delete menu;
    return out;
}

bool PanelColumn::triggerColumnHeaderMenuForTest(const QString& text)
{
    QMenu* menu = buildColumnHeaderMenu();
    bool found = false;
    for (QAction* action : menu->actions()) {
        if (!action->isSeparator() && action->text() == text) {
            action->trigger();
            found = true;
            break;
        }
    }
    delete menu;
    return found;
}

bool PanelColumn::dragSourceGroupAliveForTest() const
{
    return dragSourceGroup_ && groups_.contains(dragSourceGroup_);
}

QWidget* PanelColumn::dragHandleForTest(const QString& groupName) const
{
    PanelGroup* group = findGroupByName(groupName);
    if (!group) {
        return nullptr;
    }
    QWidget* box = stripGroupBoxFor(group);
    return box ? box->findChild<QWidget*>(QStringLiteral("panelIconGroupGrip")) : nullptr;
}

QStringList PanelColumn::compactStripGroupOrderForTest() const
{
    QStringList out;
    for (PanelGroup* group : stripGroupOrder_) {
        out << (group ? group->objectName() : QString());
    }
    return out;
}

int PanelColumn::compactStripGroupIndexForTest(const QString& panelName) const
{
    PanelGroup* owner = groupForPanel(panelName);
    return owner ? stripGroupOrder_.indexOf(owner) : -1;
}

QPoint PanelColumn::compactStripBoundaryPointForTest(int boundary) const
{
    if (!iconStrip_ || stripGroupOrder_.isEmpty()) {
        return QPoint();
    }
    auto centerX = [this](PanelGroup* group) {
        QWidget* box = stripGroupBoxFor(group);
        return box ? box->mapToGlobal(QPoint(box->width() / 2, 0)).x()
                   : iconStrip_->mapToGlobal(QPoint(iconStrip_->width() / 2, 0)).x();
    };
    auto topOf = [this](PanelGroup* group) {
        QWidget* box = stripGroupBoxFor(group);
        return box ? box->mapToGlobal(QPoint(0, 0)).y() : 0;
    };
    auto bottomOf = [this](PanelGroup* group) {
        QWidget* box = stripGroupBoxFor(group);
        return box ? box->mapToGlobal(QPoint(0, box->height())).y() : 0;
    };
    boundary = qBound(0, boundary, stripGroupOrder_.size());
    if (boundary <= 0) {
        PanelGroup* first = stripGroupOrder_.first();
        return QPoint(centerX(first), topOf(first) - 1);
    }
    if (boundary >= stripGroupOrder_.size()) {
        PanelGroup* last = stripGroupOrder_.last();
        return QPoint(centerX(last), bottomOf(last) + 1);
    }
    const int dividerIndex = boundary - 1;
    if (dividerIndex >= 0 && dividerIndex < stripDividers_.size()) {
        QWidget* divider = stripDividers_.at(dividerIndex).widget;
        if (divider) {
            return divider->mapToGlobal(QPoint(divider->width() / 2, divider->height() / 2));
        }
    }
    return QPoint();
}

bool PanelColumn::beginGroupGripDragForTest(const QString& groupName)
{
    PanelGroup* group = findGroupByName(groupName);
    QWidget* grip = dragHandleForTest(groupName);
    if (!group || !grip) {
        return false;
    }
    beginGroupDrag(group, grip->mapToGlobal(QPoint(grip->width() / 2, grip->height() / 2)));
    return true;
}

bool PanelColumn::popupStyleParityForTest(const QString& objectName) const
{
    if (!flyout_ || !flyout_->isVisible() || flyoutName_ != objectName || !flyoutGroup_) {
        return false;
    }
    // M45 C1: the popup hosts the very same `PanelGroup` widget as the docked
    // stack and the float, so its scoped style/container is shared by identity.
    const bool hosted = flyoutGroup_->parentWidget() == flyout_
                        && flyoutGroup_->containsPanel(objectName);
    // M44 C1/S2: no per-host inline override; the same scoped selectors style the
    // docked group, the float, and the flyout.
    if (!hosted || !flyout_->styleSheet().isEmpty()) {
        return false;
    }
    const QString sheet = qApp->styleSheet();
    const QString panelHex = QStringLiteral("#4d4d4d");
    const QString borderHex = QStringLiteral("#2e2e2e");
    const QString width = QString::number(Theme::kPanelBorderWidth);
    const QString container = QStringLiteral("background: ") + panelHex + QStringLiteral("; border: ")
        + width + QStringLiteral("px solid ") + borderHex;
    const bool sameContainer =
        sheet.contains(QStringLiteral("QWidget#panelIconFlyout { ") + container + QStringLiteral("; }"))
        && sheet.contains(QStringLiteral("QWidget#panelFloat { ") + container + QStringLiteral("; }"));
    const bool samePane = sheet.contains(
        QStringLiteral("QTabWidget#panelGroupTabs::pane { border: 0; background: ") + panelHex
        + QStringLiteral("; }"));
    return sameContainer && samePane;
}

QToolButton* PanelColumn::widgetMenuButtonForTest(const QString& groupObjectName) const
{
    PanelGroup* group = findGroupByName(groupObjectName);
    return group ? group->headerMenuButtonForTest() : nullptr;
}

QStringList PanelColumn::widgetMenuTextsForTest(const QString& panelName) const
{
    return PanelGroup::menuTextsForPanel(panelName);
}

bool PanelColumn::widgetMenuHasCloseForTest(const QString& panelName) const
{
    const QStringList texts = PanelGroup::menuTextsForPanel(panelName);
    return !texts.contains(QStringLiteral("Close"))
           && !texts.contains(QStringLiteral("Close Panel Group"));
}

bool PanelColumn::triggerWidgetMenuForTest(const QString& panelName, const QString& text)
{
    PanelGroup* group = groupForPanel(panelName);
    if (!group) {
        return false;
    }
    if (group->currentPanelName() != panelName) {
        showPanel(panelName, true);
        QCoreApplication::processEvents();
    }
    return group->triggerPanelMenuForTest(text);
}

QPoint PanelColumn::boundaryPointForTest(int boundary) const
{
    const QList<PanelGroup*> visible = visibleGroups();
    if (visible.isEmpty()) {
        return mapToGlobal(QPoint(4, height() / 2));
    }
    QWidget* viewport = scroll_ ? scroll_->viewport() : nullptr;
    if (!viewport) {
        return QPoint();
    }
    auto topY = [viewport](PanelGroup* group) {
        return group->mapTo(viewport, QPoint(0, 0)).y();
    };
    auto bottomY = [&topY](PanelGroup* group) { return topY(group) + group->height(); };
    int y = 0;
    if (boundary <= 0) {
        y = topY(visible.first());
    } else if (boundary >= visible.size()) {
        y = bottomY(visible.last());
    } else {
        y = (bottomY(visible.at(boundary - 1)) + topY(visible.at(boundary))) / 2;
    }
    y = qBound(0, y, viewport->height() - 1);
    return viewport->mapToGlobal(QPoint(qMax(1, viewport->width() / 2), y));
}

} // namespace pictura
