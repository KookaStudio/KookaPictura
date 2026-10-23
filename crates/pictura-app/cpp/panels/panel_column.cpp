#include "panel_column.h"

#include "panel_column_internal.h"

#include "frame.h"
#include "icons.h"
#include "panel_group.h"
#include "theme.h"

#include <QtCore/QEvent>
#include <QtCore/QJsonObject>
#include <QtCore/QMetaObject>
#include <QtCore/QRect>
#include <QtCore/QSize>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QCursor>
#include <QtGui/QFontMetrics>
#include <QtGui/QGuiApplication>
#include <QtGui/QHideEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPalette>
#include <QtGui/QResizeEvent>
#include <QtGui/QScreen>
#include <QtGui/QShowEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QBoxLayout>
#include <QtWidgets/QFrame>
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

namespace {

// M45 W8: one shared minimum width for every normal-mode widget column, so all
// columns match and none can be squeezed away. ponytail: chosen, not a sourced
// CS6 metric; tune here. The iconic strip keeps its own narrow minimum.
constexpr int kPanelMinWidth = 300;
constexpr int kPanelMaxWidth = 400;

// The invisible width-drag grip kept on a widget column's workspace edge.
constexpr int kResizeGripWidth = 4;

} // namespace

int gSharedFloor = kPanelMinWidth;

PanelColumn::PanelColumn(QWidget* parent)
    : QWidget(parent)
{
    setObjectName(QStringLiteral("panelColumnContainer"));
    setAttribute(Qt::WA_StyledBackground, true);
    auto* layout = new QVBoxLayout(this);
    // The column draws a 1 px side/bottom border (see the theme); the layout is
    // inset so the children do not paint over it.
    const int border = Theme::scaled(Theme::kPanelBorderWidth);
    layout->setContentsMargins(border, 0, border, border);
    layout->setSpacing(0);

    header_ = new QWidget(this);
    header_->setObjectName(QStringLiteral("panelColumnHeader"));
    // M47: the whole column is draggable from its top header. The filter goes on
    // the header only, never the toggle child, so a toggle click still toggles.
    header_->installEventFilter(this);
    header_->setCursor(Qt::ArrowCursor);
    header_->setToolTip(tr("Drag to move this panel column"));
    auto* headerLayout = new QHBoxLayout(header_);
    headerLayout->setContentsMargins(2, 1, 2, 1);
    headerLayout->setSpacing(2);
    headerLayout->addStretch(1);
    columnToggle_ = new QToolButton(header_);
    columnToggle_->setObjectName(QStringLiteral("panelColumnToggle"));
    columnToggle_->setAutoRaise(true);
    columnToggle_->setFixedSize(18, 18);
    connect(columnToggle_, &QToolButton::clicked, this, [this]() {
        if (toolsContent_) {
            if (toolsToggleAction_) {
                toolsToggleAction_();
            }
            updateColumnToggle();
        } else {
            setRailMode(!railMode_);
        }
    });
    headerLayout->addWidget(columnToggle_);
    layout->addWidget(header_);
    updateColumnToggle();

    scroll_ = new QScrollArea(this);
    scroll_->setObjectName(QStringLiteral("panelColumnScroll"));
    scroll_->setWidgetResizable(true);
    scroll_->setFrameShape(QFrame::NoFrame);
    // M47: the column never scrolls horizontally; the shared width floor keeps
    // content within the column instead of clipping.
    scroll_->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);

    splitter_ = new QSplitter(Qt::Vertical, scroll_);
    splitter_->setObjectName(QStringLiteral("panelColumnSplitter"));
    splitter_->setHandleWidth(Theme::scaled(Theme::kGroupDividerWidth));
    splitter_->setChildrenCollapsible(false);
    scroll_->setWidget(splitter_);
    layout->addWidget(scroll_, 1);

    iconStrip_ = new QWidget(this);
    iconStrip_->setObjectName(QStringLiteral("panelColumnIconStrip"));
    iconStrip_->setAttribute(Qt::WA_StyledBackground, true);
    iconStripLayout_ = new QVBoxLayout(iconStrip_);
    iconStripLayout_->setContentsMargins(2, 2, 2, 2);
    iconStripLayout_->setSpacing(2);
    iconStrip_->installEventFilter(this);
    iconStrip_->setVisible(false);
    layout->addWidget(iconStrip_, 1);

    // A widget column keeps a width drag even though the splitter handles are
    // 0-width: an invisible 4 px grip on its workspace-facing edge drives the
    // adjacent handle (see PicturaMainWindow::beginWidgetColumnResize).
    resizeGrip_ = new QWidget(this);
    resizeGrip_->setObjectName(QStringLiteral("panelResizeGrip"));
    resizeGrip_->setFixedWidth(Theme::scaled(kResizeGripWidth));
    resizeGrip_->setCursor(Qt::SizeHorCursor);
    resizeGrip_->setAttribute(Qt::WA_NoSystemBackground, true);
    resizeGrip_->installEventFilter(this);
    resizeGrip_->hide();

    // The thick blue insertion line, drawn by whatever is under the drag. It
    // lives in the scroll viewport so group/tab coordinates map straight in.
    indicator_ = new QWidget(scroll_->viewport());
    indicator_->setObjectName(QStringLiteral("panelDropIndicator"));
    indicator_->setAttribute(Qt::WA_TransparentForMouseEvents);
    indicator_->hide();

    // The strip's own insertion line: the scroll viewport is hidden in iconic
    // mode, so a strip drop needs a sibling inside the icon strip.
    stripIndicator_ = new QWidget(iconStrip_);
    stripIndicator_->setObjectName(QStringLiteral("panelStripDropIndicator"));
    stripIndicator_->setAttribute(Qt::WA_TransparentForMouseEvents);
    stripIndicator_->hide();

    // Phase 4: the blue region outline drawn around a whole target group for a
    // group-on-group tabify. A transparent centre keeps the group visible.
    outlineIndicator_ = new QWidget(scroll_->viewport());
    outlineIndicator_->setObjectName(QStringLiteral("panelOutlineIndicator"));
    outlineIndicator_->setAttribute(Qt::WA_TransparentForMouseEvents);
    outlineIndicator_->setAttribute(Qt::WA_StyledBackground);
    outlineIndicator_->hide();

    setMinimumHeight(0);
    updateMinimumWidth();
}

PanelColumn::~PanelColumn()
{
    // It is parented to the frame (above the splitter) so a float cannot cover
    // it; that outlives this column, so drop it here instead of leaking.
    delete edgeIndicator_;
    if (flyout_) {
        flyout_->onHidden = nullptr;
        delete flyout_;
        flyout_ = nullptr;
    }
}

PanelSide PanelColumn::side() const
{
    auto* splitter = qobject_cast<QSplitter*>(parentWidget());
    if (!splitter) {
        // A floating column has no splitter parent; fall back to the side it
        // last held while docked rather than flipping its flyout to the right.
        return lastSide_;
    }
    const int self = splitter->indexOf(const_cast<PanelColumn*>(this));
    for (int i = 0; i < splitter->count(); ++i) {
        QWidget* pane = splitter->widget(i);
        if (pane && pane->objectName() == QStringLiteral("documentTabs")) {
            lastSide_ = self < i ? PanelSide::Left : PanelSide::Right;
            return lastSide_;
        }
    }
    // The document tabs are the first pane when no left column exists.
    lastSide_ = self == 0 ? PanelSide::Left : PanelSide::Right;
    return lastSide_;
}

void PanelColumn::addGroup(PanelGroup* group)
{
    if (!group) {
        return;
    }
    // A group adopted from another column may still be wired to it; drop any
    // stale wiring to this column so `insertGroupAt` re-wires it cleanly.
    QObject::disconnect(group, nullptr, this, nullptr);
    wired_.remove(group);
    insertGroupAt(group, groups_.size());
    for (QWidget* panel : group->panels()) {
        if (panel) {
            panelVisible_[panel->objectName()] = true;
        }
    }
    if (railMode_) {
        buildIconStrip();
    }
    updateMinimumWidth();
}

void PanelColumn::adoptGroup(PanelGroup* group)
{
    if (!group) {
        return;
    }
    // M45 W4: take a group from another column. Its own tab visibility carries
    // the state (a group closed via `closeGroup` has its tabs hidden), so
    // `insertGroupAt` restores exactly the right visibility.
    QObject::disconnect(group, nullptr, this, nullptr);
    wired_.remove(group);
    insertGroupAt(group, groups_.size());
    for (QWidget* panel : group->panels()) {
        if (panel) {
            panelVisible_[panel->objectName()] = group->isPanelVisible(panel->objectName());
        }
    }
    if (railMode_) {
        buildIconStrip();
    }
    updateMinimumWidth();
}

void PanelColumn::wireGroup(PanelGroup* group)
{
    if (!group || wired_.contains(group)) {
        return;
    }
    wired_.insert(group);
    connect(group, &PanelGroup::panelActivated, this,
            [this](const QString& objectName, const QPoint& globalPos) {
                openIconFlyout(objectName, globalPos);
            });
    connect(group, &PanelGroup::tabContextMenuRequested, this,
            [this, group](const QPoint& globalPos) { showTabMenu(group, globalPos); });
    connect(group, &PanelGroup::tabDragStarted, this,
            [this, group](const QString& objectName, const QPoint& globalPos) {
                beginPanelDrag(group, objectName, globalPos);
            });
    connect(group, &PanelGroup::groupDragStarted, this,
            [this, group](const QPoint& globalPos) { beginGroupDrag(group, globalPos); });
    connect(group, &PanelGroup::dragMoved, this,
            [this](const QPoint& globalPos) { updateDrag(globalPos); });
    connect(group, &PanelGroup::dragFinished, this, [this](const QPoint& globalPos) {
        updateDrag(globalPos);
        commitDrop();
    });
    connect(group, &PanelGroup::dragCanceled, this, [this]() { cancelDrag(); });
    // M47: the per-widget menu's Close / Close Group reuse this column's own
    // close paths, the same ones `buildTabMenu` drives.
    connect(group, &PanelGroup::closePanelRequested, this, [this](const QString& objectName) {
        if (!objectName.isEmpty()) {
            showPanel(objectName, false);
        }
    });
    connect(group, &PanelGroup::closeGroupRequested, this,
            [this, group]() { closeGroup(group); });
    connect(group, &PanelGroup::flyoutCloseRequested, this,
            [this]() { closeIconFlyout(); });
}

void PanelColumn::insertGroupAt(PanelGroup* group, int index)
{
    if (!group) {
        return;
    }
    index = qBound(0, index, groups_.size());
    groups_.insert(index, group);
    group->setParent(nullptr);
    splitter_->insertWidget(index, group);
    splitter_->setStretchFactor(index, 1);
    group->setFloating(false);
    group->setFlyout(false);
    group->setVisible(!group->visibleTitles().isEmpty());
    wireGroup(group);
}

bool PanelColumn::removeGroup(PanelGroup* group)
{
    const int index = groups_.indexOf(group);
    if (index < 0) {
        return false;
    }
    groups_.removeAt(index);
    group->setParent(nullptr);
    return true;
}

PanelGroup* PanelColumn::takeGroup(PanelGroup* group)
{
    if (!group) {
        return nullptr;
    }
    const int index = groups_.indexOf(group);
    if (index < 0) {
        return nullptr;
    }
    groups_.removeAt(index);
    group->setParent(nullptr);
    QObject::disconnect(group, nullptr, this, nullptr);
    wired_.remove(group);
    return group;
}

PanelGroup* PanelColumn::takeGroup(const QString& groupObjectName)
{
    PanelGroup* group = findGroupByName(groupObjectName);
    if (!group) {
        return nullptr;
    }
    const int index = groups_.indexOf(group);
    if (index < 0) {
        return nullptr;
    }
    groups_.removeAt(index);
    group->setParent(nullptr);
    // Leave the group unwired here; the adopting column re-wires it.
    QObject::disconnect(group, nullptr, this, nullptr);
    wired_.remove(group);
    return group;
}

void PanelColumn::cleanupEmptyGroup(PanelGroup* group)
{
    if (!group) {
        return;
    }
    if (group->titleCountForTest() > 0) {
        // M47: a group whose tabs are all hidden is empty on screen; hide it
        // but keep it alive (and float ownership intact) so a later show works.
        if (group->visibleTitles().isEmpty()) {
            group->setVisible(false);
            if (group == flyoutGroup_) {
                closeIconFlyout();
            }
            refreshFloorAfterContentChange();
            maybeRemoveSelf();
        }
        return;
    }
    if (group == flyoutGroup_) {
        closeIconFlyout();
    }
    wired_.remove(group);
    if (PanelFloat* floatWindow = floatForGroup(group)) {
        destroyFloat(floatWindow);
        refreshFloorAfterContentChange();
        maybeRemoveSelf();
        return;
    }
    removeGroup(group);
    group->deleteLater();
    refreshFloorAfterContentChange();
    maybeRemoveSelf();
}

PanelFloat* PanelColumn::floatForGroup(PanelGroup* group) const
{
    for (PanelFloat* floatWindow : floats_) {
        if (floatWindow && floatWindow->group() == group) {
            return floatWindow;
        }
    }
    return nullptr;
}

PanelFloat* PanelColumn::floatForPanel(const QString& objectName) const
{
    for (PanelFloat* floatWindow : floats_) {
        PanelGroup* group = floatWindow ? floatWindow->group() : nullptr;
        if (group && group->containsPanel(objectName)) {
            return floatWindow;
        }
    }
    return nullptr;
}

void PanelColumn::refreshFloorAfterContentChange()
{
    updateMinimumWidth();
    if (PicturaMainWindow* frame = owningFrame()) {
        refreshSharedFloor(frame);
    }
}

PanelGroup* PanelColumn::findGroupByName(const QString& objectName) const
{
    for (PanelGroup* group : groups_) {
        if (group && group->objectName() == objectName) {
            return group;
        }
    }
    return nullptr;
}

QList<PanelGroup*> PanelColumn::visibleGroups() const
{
    QList<PanelGroup*> out;
    for (PanelGroup* group : groups_) {
        if (group && group->isVisible()) {
            out << group;
        }
    }
    return out;
}

PanelGroup* PanelColumn::groupForPanel(const QString& objectName) const
{
    for (PanelGroup* group : groups_) {
        if (group && group->containsPanel(objectName)) {
            return group;
        }
    }
    return nullptr;
}

bool PanelColumn::showPanel(const QString& objectName, bool visible)
{
    PanelGroup* group = groupForPanel(objectName);
    PanelFloat* hostFloat = group ? nullptr : floatForPanel(objectName);
    if (hostFloat) {
        group = hostFloat->group();
    }
    if (!group) {
        panelVisible_[objectName] = visible;
        return true;
    }
    if (!group->setPanelVisible(objectName, visible)) {
        return false;
    }
    panelVisible_[objectName] = visible;
    if (hostFloat) {
        // A floated panel has no docked group. Hiding the last visible tab closes
        // the overlay (re-docking the group through the one close path); a
        // still-populated overlay just shrinks to its new content.
        if (!visible && group->visibleTitles().isEmpty()) {
            closeFloat(hostFloat);
        } else {
            hostFloat->syncToContent();
        }
        refreshFloorAfterContentChange();
        emit stateChanged();
        return true;
    }
    if (visible) {
        show();
        group->setVisible(true);
    }
    if (railMode_) {
        buildIconStrip();
    }
    refreshFloorAfterContentChange();
    emit stateChanged();
    if (!visible) {
        if (group == flyoutGroup_ && group->visibleTitles().isEmpty()) {
            closeIconFlyout();
        }
        maybeRemoveSelf();
    }
    return true;
}

bool PanelColumn::isPanelVisible(const QString& objectName) const
{
    return panelVisible_.value(objectName, false);
}

void PanelColumn::closeGroup(PanelGroup* group)
{
    if (!group) {
        return;
    }
    // A group hosted in the flyout still reports its origin float. Restore it
    // first so the close acts on the live presentation, not the empty origin.
    if (group == flyoutGroup_) {
        closeIconFlyout();
    }
    // A floated group has left `groups_`; closing it must tear the overlay down
    // too, or it is left as an empty ghost with no way to restore it.
    if (PanelFloat* hostFloat = floatForGroup(group)) {
        closeFloat(hostFloat);
        return;
    }
    for (QWidget* panel : group->panels()) {
        if (panel) {
            const QString name = panel->objectName();
            // M45 W4: hide each tab too, so the group stays closed if it is
            // rehomed to another column when this dynamic column is removed.
            group->setPanelVisible(name, false);
            panelVisible_[name] = false;
        }
    }
    group->setVisible(false);
    refreshFloorAfterContentChange();
    emit stateChanged();
    if (group == flyoutGroup_) {
        closeIconFlyout();
    }
    maybeRemoveSelf();
}

void PanelColumn::maybeRemoveSelf()
{
    // M45 W4: the frame owns the "is this dynamic column empty" test; every
    // path that can empty this column funnels through here.
    if (auto* frame = owningFrame()) {
        frame->removeColumnIfEmpty(this);
    }
}

PicturaMainWindow* PanelColumn::owningFrame() const
{
    QWidget* top = window();
    if (auto* frame = qobject_cast<PicturaMainWindow*>(top)) {
        return frame;
    }
    // A column hosted in a floating overlay: the overlay's parent is the frame.
    if (top) {
        if (auto* frame = qobject_cast<PicturaMainWindow*>(top->parentWidget())) {
            return frame;
        }
    }
    return nullptr;
}

void PanelColumn::setToolsContent(QWidget* content, std::function<int()> columnsState,
                                  std::function<void()> onToggleRequested)
{
    if (!content || toolsContent_) {
        return;
    }
    toolsContent_ = content;
    toolsColumnsState_ = std::move(columnsState);
    toolsToggleAction_ = std::move(onToggleRequested);
    // No group stack and no rail: host the plain content in a scroll area so
    // the toolbox's scaled height cannot set the frame's minimum height.
    scroll_->setVisible(false);
    iconStrip_->setVisible(false);
    toolsScroll_ = new QScrollArea(this);
    toolsScroll_->setObjectName(QStringLiteral("toolsScroll"));
    toolsScroll_->setWidgetResizable(true);
    toolsScroll_->setFrameShape(QFrame::NoFrame);
    toolsScroll_->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    toolsScroll_->setWidget(content);
    if (auto* box = qobject_cast<QBoxLayout*>(layout())) {
        box->insertWidget(1, toolsScroll_, 1);
    }
    content->setVisible(true);
    updateMinimumWidth();
    updateColumnToggle();
    updateResizeGrip();
}

void PanelColumn::refreshToolsWidth()
{
    if (!toolsContent_) {
        return;
    }
    updateMinimumWidth();
    // A whole-column overlay hugs the tool grid; the 1<->2 column flip changed
    // that size, so re-fit the overlay to the content. The flip rebuilds the
    // grid synchronously, but the layout chain has not measured the new
    // arrangement yet, so the re-fit is deferred one tick — otherwise the
    // overlay takes the previous mode's height. The float is the timer's
    // context, so a float destroyed before the tick cancels the callback.
    if (columnFloat_) {
        PanelFloat* floatWindow = columnFloat_;
        QTimer::singleShot(0, floatWindow, [floatWindow]() { floatWindow->syncToContent(); });
        return;
    }
    // A splitter pane keeps its explicit size until told otherwise; resize this
    // pane to its content width when a 1<->2 column flip changed that width.
    if (auto* splitter = qobject_cast<QSplitter*>(parentWidget())) {
        const int index = splitter->indexOf(this);
        if (index >= 0 && splitter->width() > 0) {
            QList<int> sizes = splitter->sizes();
            if (index < sizes.size()) {
                sizes[index] = minimumWidth();
                splitter->setSizes(sizes);
            }
        }
    }
}

void PanelColumn::updateMinimumWidth()
{
    if (toolsContent_) {
        // D2: the tools column tracks its content width (slot grid + fg/bg),
        // not the widget-column floor. It is fixed-size in both directions: the
        // maximum equals the minimum, so neither a splitter-handle drag (the
        // handle beside it is disabled too) nor a stray programmatic resize can
        // change the toolbar's width.
        const int want = (toolsContent_->minimumWidth() > 0 ? toolsContent_->minimumWidth()
                                                            : toolsContent_->sizeHint().width())
                         + 2 * Theme::scaled(Theme::kPanelBorderWidth);
        setMinimumWidth(qMax(Theme::scaled(kIconStripMinWidth), want));
        setMaximumWidth(qMax(Theme::scaled(kIconStripMinWidth), want));
        return;
    }
    if (railMode_) {
        setMinimumWidth(Theme::scaled(kIconStripMinWidth));
        return;
    }
    // M47: derive a shared floor from this column's real content (never from a
    // sibling splitter), clamped so all normal columns converge and none can
    // grow past the max.
    const int minWidth = minNormalWidth();
    const int maxWidth = maxNormalWidth();
    int need = minWidth;
    if (splitter_) {
        need = qMax(need, splitter_->minimumSizeHint().width());
    }
    if (scroll_) {
        need += scroll_->verticalScrollBar()->sizeHint().width() + 2 * scroll_->frameWidth();
    }
    gSharedFloor = qBound(minWidth, qMax(gSharedFloor, need), maxWidth);
    setMinimumWidth(gSharedFloor);
}

void PanelColumn::refreshSharedFloor(PicturaMainWindow* frame)
{
    gSharedFloor = minNormalWidth();
    if (!frame) {
        return;
    }
    for (PanelColumn* column : frame->allPanelColumns()) {
        column->updateMinimumWidth();
    }
}

void PanelColumn::setAutoCollapseIconic(bool on)
{
    if (autoCollapseIconic_ == on) {
        return;
    }
    autoCollapseIconic_ = on;
    emit stateChanged();
}

void PanelColumn::setAutoShowHidden(bool on)
{
    if (autoShowHidden_ == on) {
        return;
    }
    autoShowHidden_ = on;
    if (railMode_) {
        buildIconStrip();
    }
    emit stateChanged();
}

void PanelColumn::applyUiScale()
{
    if (QLayout* columnLayout = layout()) {
        const int border = Theme::scaled(Theme::kPanelBorderWidth);
        columnLayout->setContentsMargins(border, 0, border, border);
    }
    if (columnToggle_) {
        const int side = Theme::scaled(18);
        columnToggle_->setFixedSize(side, side);
    }
    for (PanelGroup* group : groups_) {
        if (group) {
            group->applyUiScale();
        }
    }
    if (railMode_) {
        buildIconStrip();
    }
    updateColumnToggle();
    updateMinimumWidth();
    updateResizeGrip();
}

void PanelColumn::setPreferredWidth(int width)
{
    if (width <= 0) {
        return;
    }
    // A normal column is never wider than `kMaxNormalWidth`; an iconic strip
    // keeps its own narrow floor. This is both the layout bound and the guard
    // against a stale oversized store expanding the column across the workspace.
    const int floor = railMode_ ? Theme::scaled(kIconStripMinWidth) : minNormalWidth();
    const int target = qBound(floor, width, maxNormalWidth());
    auto* splitter = qobject_cast<QSplitter*>(parentWidget());
    const int index = splitter ? splitter->indexOf(this) : -1;
    if (!splitter || index < 0 || splitter->width() <= 0) {
        // Before the first layout the splitter has no width; remember the value
        // and apply it on show.
        pendingWidth_ = target;
        return;
    }
    QList<int> sizes = splitter->sizes();
    if (index >= sizes.size()) {
        pendingWidth_ = target;
        return;
    }
    // Keep every other pane's size (other columns, the document tabs) and set
    // only this column. The M41 two-pane case is the same operation.
    sizes[index] = target;
    splitter->setSizes(sizes);
    pendingWidth_ = 0;
}

void PanelColumn::setRestoredWidth(int width)
{
    if (width <= 0) {
        return;
    }
    if (railMode_) {
        normalWidthBeforeIconic_ = qBound(minNormalWidth(), width, maxNormalWidth());
        return;
    }
    setPreferredWidth(width);
}

void PanelColumn::showEvent(QShowEvent* event)
{
    QWidget::showEvent(event);
    if (pendingWidth_ > 0 && !railMode_) {
        const int width = pendingWidth_;
        pendingWidth_ = 0;
        setPreferredWidth(width);
    }
    updateResizeGrip();
}

void PanelColumn::updateResizeGrip()
{
    if (!resizeGrip_) {
        return;
    }
    // Only a docked widget column has a workspace edge to drag against.
    if (toolsContent_ || !isVisible() || !qobject_cast<QSplitter*>(parentWidget())) {
        resizeGrip_->hide();
        return;
    }
    const bool left = side() == PanelSide::Left;
    resizeGrip_->setGeometry(left ? width() - kResizeGripWidth : 0, 0, kResizeGripWidth,
                             height());
    resizeGrip_->show();
    resizeGrip_->raise();
}

void PanelColumn::resizeEvent(QResizeEvent* event)
{
    QWidget::resizeEvent(event);
    // A normal-mode resize lands the flip's widening; the live width is now the
    // real normal width, so `persistedWidth` may use it again.
    if (!railMode_ && width() > 0) {
        widthFlipPending_ = false;
    }
    updateResizeGrip();
}

void PanelColumn::setBorderEdgeSuppressed(bool left, bool right)
{
    auto* l = layout();
    if (!l) {
        return;
    }
    const int b = Theme::scaled(Theme::kPanelBorderWidth);
    l->setContentsMargins(left ? 0 : b, 0, right ? 0 : b, b);
}

QJsonArray PanelColumn::savePanelState() const
{
    auto serialize = [this](PanelGroup* group, bool floating) {
        QJsonObject entry;
        entry.insert(QStringLiteral("name"), group->objectName());
        QJsonArray order;
        QJsonArray visible;
        for (QWidget* panel : group->panels()) {
            if (!panel) {
                continue;
            }
            order.append(panel->objectName());
            if (group->isPanelVisible(panel->objectName())) {
                visible.append(panel->objectName());
            }
        }
        entry.insert(QStringLiteral("order"), order);
        entry.insert(QStringLiteral("visible"), visible);
        entry.insert(QStringLiteral("minimized"), group->isMinimized());
        entry.insert(QStringLiteral("collapsed"),
                     group == flyoutGroup_ ? flyoutWasCollapsed_
                                           : group->isCollapsedToIcons());
        // A floated group has left `groups_`; record it additively so its panels
        // are kept visible on restore without hiding the group's docked siblings.
        if (floating) {
            entry.insert(QStringLiteral("floating"), true);
        }
        return entry;
    };

    QJsonArray groups;
    for (PanelGroup* group : groups_) {
        if (!group) {
            continue;
        }
        // A flyout hosted from a floating row is serialized through its origin
        // float below, not twice as a docked group.
        if (group == flyoutGroup_ && flyoutFloat_) {
            continue;
        }
        groups.append(serialize(group, false));
    }
    for (PanelFloat* floatWindow : floats_) {
        PanelGroup* group = floatWindow ? floatWindow->group() : nullptr;
        if (group) {
            QJsonObject entry = serialize(group, true);
            const QRect rect(floatWindow->mapToGlobal(QPoint(0, 0)), floatWindow->size());
            QJsonArray geometry;
            geometry.append(rect.x());
            geometry.append(rect.y());
            geometry.append(rect.width());
            geometry.append(rect.height());
            entry.insert(QStringLiteral("floatGeometry"), geometry);
            groups.append(entry);
        }
    }
    return groups;
}

void PanelColumn::restorePanelState(const QJsonArray& state)
{
    for (const QJsonValue& value : state) {
        const QJsonObject entry = value.toObject();
        const QJsonArray order = entry.value(QStringLiteral("order")).toArray();
        QStringList visibleList;
        for (const QJsonValue& name : entry.value(QStringLiteral("visible")).toArray()) {
            visibleList << name.toString();
        }
        // A floated group has no docked identity to find by name (a torn-off
        // single panel's group is named after its panel). Apply it additively to
        // whichever group now owns each panel, so the panel does not vanish.
        if (entry.value(QStringLiteral("floating")).toBool(false)) {
            for (const QString& name : visibleList) {
                if (PanelGroup* group = groupForPanel(name)) {
                    group->setPanelVisible(name, true);
                    panelVisible_[name] = true;
                }
            }
            continue;
        }
        PanelGroup* group = findGroupByName(entry.value(QStringLiteral("name")).toString());
        if (!group && !order.isEmpty()) {
            group = groupForPanel(order.first().toString());
        }
        if (!group) {
            continue;
        }
        QStringList orderList;
        for (const QJsonValue& name : order) {
            orderList << name.toString();
        }
        if (!orderList.isEmpty()) {
            group->setPanelOrder(orderList);
        }
        for (QWidget* panel : group->panels()) {
            if (!panel) {
                continue;
            }
            const bool visible = visibleList.contains(panel->objectName());
            group->setPanelVisible(panel->objectName(), visible);
            panelVisible_[panel->objectName()] = visible;
        }
        group->setMinimized(entry.value(QStringLiteral("minimized")).toBool(false));
        group->setCollapsedToIcons(entry.value(QStringLiteral("collapsed")).toBool(false));
        // M44: `setPanelVisible` makes each shown panel current, so the last
        // visible panel would win; restore the first visible as the active tab.
        group->setCurrentToFirstVisible();
    }
    if (railMode_) {
        buildIconStrip();
    }
    updateMinimumWidth();
}

} // namespace pictura

#include "moc_panel_column.cpp"
