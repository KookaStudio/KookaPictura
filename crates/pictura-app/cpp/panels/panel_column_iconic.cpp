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

constexpr int kIconLabelMinWidth = 16;
constexpr int kIconButtonSize = 34;
constexpr int kIconPixmapSize = 24;
// The divider is kept as a layout/drop marker but draws nothing: a zero-height
// frame with a transparent rule. The 1 px between groups is each group's own
// bottom rule. ponytail: chosen, not a sourced CS6 metric.
constexpr int kCompactDividerHeight = 0;

} // namespace

void PanelColumn::setRailMode(bool iconic)
{
    // A tools column has no iconic rail mode; its toggle reflows the content.
    if (toolsContent_) {
        updateColumnToggle();
        return;
    }
    if (railMode_ == iconic) {
        updateColumnToggle();
        return;
    }
    railMode_ = iconic;
    if (iconic) {
        widthFlipPending_ = false;
        if (width() > 0) {
            normalWidthBeforeIconic_ = width();
        }
        buildIconStrip();
        iconStrip_->setVisible(true);
        scroll_->setVisible(false);
        updateMinimumWidth();
        // M49: the compact column stays a freely resizable splitter pane (the
        // neighbour drag can widen it), not a fixed-width strip.
        setMaximumWidth(QWIDGETSIZE_MAX);
        setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Expanding);
        setPreferredWidth(Theme::scaled(kIconStripMinWidth));
    } else {
        closeIconFlyout();
        iconStrip_->setVisible(false);
        scroll_->setVisible(true);
        setMaximumWidth(QWIDGETSIZE_MAX);
        setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Preferred);
        updateMinimumWidth();
        // The widening below can stay pending until layout; remember that so a
        // save in the meantime writes the normal width, not the strip width.
        widthFlipPending_ = true;
        setPreferredWidth(normalWidthBeforeIconic_ > 0 ? normalWidthBeforeIconic_
                                                       : minimumWidth());
    }
    updateColumnToggle();
    // Re-place the workspace-edge width grip for the new mode/width (a rail
    // column is resizable from its workspace-facing edge like a normal one).
    updateResizeGrip();
    // A whole-column overlay snaps to the icon strip's content when the rail
    // mode flips while the column is floating.
    if (columnFloat_) {
        columnFloat_->syncToContent();
    }
    emit stateChanged();
}

void PanelColumn::updateColumnToggle()
{
    if (toolsContent_) {
        // D2: the tools toggle shows the icon for the action it performs — the
        // two-column icon while one tool column is shown, and vice versa.
        const bool two = toolsColumnsState_ && toolsColumnsState_() == 2;
        const QIcon target =
            icon(two ? QStringLiteral("panel.columnsOne") : QStringLiteral("panel.columnsTwo"));
        if (!target.isNull()) {
            columnToggle_->setIcon(target);
            columnToggle_->setIconSize(QSize(Theme::scaled(16), Theme::scaled(16)));
            columnToggle_->setText(QString());
        } else {
            columnToggle_->setIcon(QIcon());
            columnToggle_->setText(two ? QStringLiteral("\u00ab") : QStringLiteral("\u00bb"));
        }
        columnToggle_->setToolTip(tr("Toggle one or two columns of tools"));
        return;
    }
    // M44: the toggle shows the icon for the action it performs — collapse to
    // icons in normal mode (`panel.columnsTwo`, the inward double chevron) and
    // expand in iconic mode (`panel.columnsOne`).
    const QIcon target = icon(railMode_ ? QStringLiteral("panel.columnsOne")
                                        : QStringLiteral("panel.columnsTwo"));
    if (!target.isNull()) {
        columnToggle_->setIcon(target);
        columnToggle_->setIconSize(QSize(Theme::scaled(16), Theme::scaled(16)));
        columnToggle_->setText(QString());
    } else {
        columnToggle_->setIcon(QIcon());
        columnToggle_->setText(railMode_ ? QStringLiteral("\u00ab") : QStringLiteral("\u00bb"));
    }
    columnToggle_->setToolTip(railMode_ ? tr("Expand panels")
                                        : tr("Collapse panels to icons"));
}

void PanelColumn::clearIconStrip()
{
    stripLabels_.clear();
    stripEntries_.clear();
    stripGroupOrder_.clear();
    stripGroupBoxes_.clear();
    stripDividers_.clear();
    stripPressPending_ = false;
    stripDragging_ = false;
    stripDragButton_ = nullptr;
    stripGripGroup_ = nullptr;
    while (QLayoutItem* item = iconStripLayout_->takeAt(0)) {
        if (QWidget* widget = item->widget()) {
            delete widget;
        }
        delete item;
    }
}

void PanelColumn::buildIconStrip()
{
    clearIconStrip();
    bool first = true;
    for (PanelGroup* group : groups_) {
        // `Auto-Show Hidden Panels` includes the hidden panels' icons in the
        // strip; otherwise only the visible panels of visible groups appear.
        const QList<QWidget*> panels =
            autoShowHidden_ ? group->panels() : group->visiblePanels();
        if (panels.isEmpty()) {
            continue;
        }
        if (!first) {
            auto* divider = new QFrame(iconStrip_);
            divider->setObjectName(QStringLiteral("panelIconDivider"));
            divider->setAttribute(Qt::WA_StyledBackground, true);
            divider->setFrameShape(QFrame::NoFrame);
            divider->setFixedHeight(kCompactDividerHeight);
            iconStripLayout_->addWidget(divider);
            stripDividers_.append(StripDivider{group, divider});
        }
        first = false;
        // M44 C3: one shared container per group (with a drag-handle grip above
        // its icons) so the group reads as a single visual unit and the whole
        // group is draggable.
        auto* groupBox = new QWidget(iconStrip_);
        groupBox->setObjectName(QStringLiteral("panelIconGroup"));
        groupBox->setAttribute(Qt::WA_StyledBackground, true);
        groupBox->setProperty("groupObjectName", group->objectName());
        auto* groupLayout = new QVBoxLayout(groupBox);
        groupLayout->setContentsMargins(1, 1, 1, 1);
        groupLayout->setSpacing(1);
        groupLayout->addWidget(makeGroupGrip(group));
        const QStringList titles = autoShowHidden_ ? group->titles() : group->visibleTitles();
        for (int i = 0; i < panels.size(); ++i) {
            QWidget* panel = panels.at(i);
            const QString title = titles.value(i);
            auto* row = new QWidget(groupBox);
            row->setObjectName(QStringLiteral("panelIconRow"));
            auto* rowLayout = new QHBoxLayout(row);
            rowLayout->setContentsMargins(0, 0, 0, 0);
            rowLayout->setSpacing(4);
            QToolButton* iconButton = makeIconButton(
                row, panel->objectName(), title, group->iconForPanel(panel->objectName()));
            rowLayout->addWidget(iconButton);
            stripEntries_.append(StripEntry{group, panel->objectName(), iconButton});
            auto* label = new QLabel(title, row);
            label->setObjectName(QStringLiteral("panelIconLabel"));
            label->setProperty("fullTitle", title);
            label->setProperty("iconButton", QVariant::fromValue<QObject*>(iconButton));
            label->setCursor(Qt::PointingHandCursor);
            label->installEventFilter(this);
            label->setVisible(iconLabelsShown_);
            rowLayout->addWidget(label);
            rowLayout->addStretch(1);
            groupLayout->addWidget(row);
            stripLabels_ << label;
        }
        iconStripLayout_->addWidget(groupBox);
        stripGroupOrder_.append(group);
        stripGroupBoxes_.insert(group, groupBox);
    }
    iconStripLayout_->addStretch(1);
    updateIconStripLabels();
}

QWidget* PanelColumn::makeGroupGrip(PanelGroup* group)
{
    auto* grip = new QLabel(iconStrip_);
    grip->setObjectName(QStringLiteral("panelIconGroupGrip"));
    grip->setProperty("groupObjectName", group ? group->objectName() : QString());
    grip->setAlignment(Qt::AlignCenter);
    grip->setText(QStringLiteral("\u2022\u2022\u2022"));
    grip->setFixedHeight(Theme::scaled(kCompactGripHeight));
    grip->setCursor(Qt::SizeAllCursor);
    grip->setToolTip(tr("Drag to move this panel group"));
    grip->setAttribute(Qt::WA_StyledBackground, true);
    grip->installEventFilter(this);
    return grip;
}

QWidget* PanelColumn::stripGroupBoxFor(PanelGroup* group) const
{
    return stripGroupBoxes_.value(group, nullptr);
}

void PanelColumn::updateIconStripLabels()
{
    // M44: show the label as soon as the row has room beyond the icon button and
    // elide its text to that room, instead of hiding it until the full title
    // fits (the M42/M43 `>= 120 px` gate).
    const int available = iconStrip_->width() - Theme::scaled(kIconButtonSize) - 10;
    const bool show = available >= Theme::scaled(kIconLabelMinWidth);
    QFontMetrics metrics(font());
    for (QLabel* label : stripLabels_) {
        if (!label) {
            continue;
        }
        if (show) {
            const QString full = label->property("fullTitle").toString();
            label->setText(metrics.elidedText(full, Qt::ElideRight, available));
        }
        label->setVisible(show);
    }
    iconLabelsShown_ = show;
}

QToolButton* PanelColumn::makeIconButton(QWidget* parent, const QString& objectName,
                                         const QString& title, const QIcon& icon)
{
    auto* button = new QToolButton(parent);
    button->setObjectName(QStringLiteral("panelIcon_") + objectName);
    if (!icon.isNull()) {
        button->setIcon(icon);
        button->setIconSize(QSize(Theme::scaled(kIconPixmapSize), Theme::scaled(kIconPixmapSize)));
    } else {
        button->setText(title.left(1));
    }
    button->setToolTip(title);
    button->setAutoRaise(true);
    button->setCheckable(true);
    button->setChecked(activeIconName_ == objectName);
    button->setFixedSize(Theme::scaled(kIconButtonSize), Theme::scaled(kIconButtonSize));
    button->installEventFilter(this);
    connect(button, &QToolButton::clicked, this, [this, objectName, button]() {
        openIconFlyout(objectName, button->mapToGlobal(QPoint(button->width(), 0)));
    });
    return button;
}

void PanelColumn::ensureFlyout()
{
    if (flyout_) {
        return;
    }
    // The flyout is parented to the frame so a child-mode overlay can extend
    // across the workspace rather than being clipped to this column.
    QWidget* host = owningFrame();
    if (!host) {
        host = this;
    }
    flyout_ = new PanelFlyout(host);
    flyout_->setObjectName(QStringLiteral("panelIconFlyout"));
    connect(flyout_, &QObject::destroyed, this, [this](QObject* object) {
        if (flyout_ == object) {
            flyout_ = nullptr;
            flyoutLayout_ = nullptr;
        }
    });
    flyout_->setMinimumWidth(PanelFloat::kFloatMinWidth);
    // M45 C1: the clicked group is reparented in as-is, so its own tab bar (and
    // the per-widget `▾` menu) is the flyout chrome — no bespoke one-tab header,
    // no parity difference from docked.
    flyoutLayout_ = new QVBoxLayout(flyout_);
    flyoutLayout_->setContentsMargins(1, 1, 1, 1);
    flyoutLayout_->setSpacing(0);
    flyout_->onHidden = [this]() { restoreFlyoutGroup(); };
}

void PanelColumn::openIconFlyout(const QString& objectName, const QPoint& globalPos)
{
    // The same icon toggles its flyout closed; another icon in the same column
    // replaces it. A column owns at most one flyout.
    if (flyout_ && flyout_->isVisible() && flyoutName_ == objectName) {
        closeIconFlyout();
        return;
    }
    closeIconFlyout();
    PanelGroup* group = resolveFlyoutGroup(objectName);
    if (!group) {
        return;
    }
    // `Auto-Show Hidden Panels`: opening a hidden panel's icon reveals it.
    if (autoShowHidden_ && !group->isPanelVisible(objectName)) {
        panelVisible_[objectName] = true;
        group->setPanelVisible(objectName, true);
    }
    // The clicked panel is the group's active tab.
    group->setCurrentPanel(objectName);
    ensureFlyout();
    flyoutGroup_ = group;
    flyoutName_ = objectName;
    flyoutGroupIndex_ = groups_.indexOf(group);
    // Phase 7: a compact icon in a float opens the same flyout; remember the
    // overlay to return the group to on close.
    flyoutFloat_ = floatForGroup(group);
    // Capture the anchor before the group is reparented: a floating row's icon
    // button moves into the flyout with the group, so it cannot be used later.
    if (PanelFloat* origin = flyoutFloat_) {
        flyoutAnchorRect_ =
            QRect(origin->mapToGlobal(QPoint(0, 0)), origin->size());
    } else if (QToolButton* button = flyoutButtonFor(objectName)) {
        flyoutAnchorRect_ =
            QRect(button->mapToGlobal(QPoint(0, 0)), button->size());
    } else {
        flyoutAnchorRect_ =
            QRect(globalPos - QPoint(Theme::scaled(kIconButtonSize), 0),
                  QSize(Theme::scaled(kIconButtonSize), Theme::scaled(kIconButtonSize)));
    }
    // A collapsed group's icon row is not the panel content the flyout shows;
    // remember the state and expand it only after it is in the flyout, so the
    // origin float is not expanded into a ghost.
    flyoutWasCollapsed_ = group->isCollapsedToIcons();
    // Detach the whole group from the column splitter, then host it in the
    // flyout: one reparent out, one back on close, never two parents.
    group->setParent(nullptr);
    flyoutLayout_->addWidget(group);
    if (flyoutWasCollapsed_) {
        group->setCollapsedToIcons(false);
    }
    // A floating group keeps its own top bar while collapsed; the flyout is the
    // only header while it hosts the group.
    group->setFloating(false);
    group->setFlyout(true);
    group->setVisible(true);
    if (PanelFloat* origin = flyoutFloat_) {
        origin->hide();
    }
    setActiveIcon(objectName);
    QSize size = group->sizeHint().expandedTo(QSize(PanelFloat::kFloatMinWidth, 180));
    if (size.width() > 480) {
        size.setWidth(480);
    }
    flyout_->setResizeEdge(flyoutSide() == QStringLiteral("left"));
    flyout_->resize(size);
    placeFlyout(flyoutAnchorRect_, flyout_->size());
    flyout_->show();
    flyout_->raise();
    if (PanelFloat::overlayUsesTopLevel()) {
        // The History panel's shortcuts need keyboard focus in the flyout.
        flyout_->activateWindow();
    }
}

PanelGroup* PanelColumn::resolveFlyoutGroup(const QString& objectName) const
{
    if (PanelGroup* group = groupForPanel(objectName)) {
        return group;
    }
    // Phase 7: a compact icon row belongs to a live float, whose group has left
    // `groups_`. The flyout hosts that same group; it is not a second flyout.
    for (PanelFloat* floatWindow : floats_) {
        PanelGroup* group = floatWindow ? floatWindow->group() : nullptr;
        if (group && group->containsPanel(objectName)) {
            return group;
        }
    }
    return nullptr;
}

QString PanelColumn::flyoutSide() const
{
    // M43: the inner side generalises to N columns from the column's own
    // layout side (right column => flyout to its left, left column => to its
    // right), not from the window-edge heuristic.
    return side() == PanelSide::Right ? QStringLiteral("left") : QStringLiteral("right");
}

void PanelColumn::placeFlyout(const QRect& buttonGlobalRect, const QSize& size)
{
    if (!flyout_) {
        return;
    }
    // M43: use the button's actual global rect. For a right-hand column the
    // flyout sits flush to the left of the button (its inner edge); for a
    // left-hand column, to the right. The screen clamp bounds only the inner
    // coordinate, so it can never push the flyout across to the outer side.
    const bool innerLeft = flyoutSide() == QStringLiteral("left");
    int x = innerLeft ? buttonGlobalRect.left() - size.width()
                      : buttonGlobalRect.right() + 1;
    int y = buttonGlobalRect.top();
    QScreen* screen = window() ? window()->screen()
                               : QGuiApplication::screenAt(buttonGlobalRect.center());
    if (!screen) {
        screen = QGuiApplication::primaryScreen();
    }
    if (screen) {
        const QRect avail = screen->availableGeometry();
        x = qBound(avail.left(), x, avail.right() - size.width() + 1);
        y = qBound(avail.top(), y, avail.bottom() - size.height() + 1);
    }
    if (PanelFloat::overlayUsesTopLevel()) {
        flyout_->move(x, y);
        return;
    }
    // A child overlay moves in parent-relative coordinates and is clipped to
    // the frame rect, so convert and clamp there.
    QWidget* host = flyout_->parentWidget();
    if (!host) {
        return;
    }
    const QPoint local = host->mapFromGlobal(QPoint(x, y));
    const int maxX = qMax(0, host->width() - size.width());
    const int maxY = qMax(0, host->height() - size.height());
    flyout_->move(qBound(0, local.x(), maxX), qBound(0, local.y(), maxY));
}

void PanelColumn::reclampFlyout()
{
    if (!flyout_ || !flyout_->isVisible()) {
        return;
    }
    if (!PanelFloat::overlayUsesTopLevel()) {
        QWidget* host = flyout_->parentWidget();
        if (host) {
            const QSize size = flyout_->size().boundedTo(host->size()).expandedTo(
                flyout_->minimumSize());
            flyout_->resize(size);
        }
    }
    placeFlyout(flyoutAnchorRect_, flyout_->size());
}

QToolButton* PanelColumn::flyoutButtonFor(const QString& objectName) const
{
    if (QToolButton* button = stripButtonFor(objectName)) {
        return button;
    }
    for (PanelGroup* group : groups_) {
        if (!group) {
            continue;
        }
        if (QToolButton* button =
                group->findChild<QToolButton*>(QStringLiteral("panelGroupIcon_") + objectName)) {
            return button;
        }
    }
    // Phase 7: the compact icon may belong to a live float rather than a docked
    // group; anchor the flyout to that button too.
    for (PanelFloat* floatWindow : floats_) {
        PanelGroup* group = floatWindow ? floatWindow->group() : nullptr;
        if (!group) {
            continue;
        }
        if (QToolButton* button =
                group->findChild<QToolButton*>(QStringLiteral("panelGroupIcon_") + objectName)) {
            return button;
        }
    }
    return nullptr;
}

void PanelColumn::setActiveIcon(const QString& objectName)
{
    activeIconName_ = objectName;
    if (!iconStrip_) {
        return;
    }
    for (QToolButton* button : iconStrip_->findChildren<QToolButton*>()) {
        if (button && button->objectName().startsWith(QStringLiteral("panelIcon_"))) {
            button->setChecked(
                button->objectName().mid(QStringLiteral("panelIcon_").size()) == objectName);
        }
    }
}

void PanelColumn::closeIconFlyout()
{
    if (flyout_ && flyout_->isVisible()) {
        flyout_->hide();
    } else {
        restoreFlyoutGroup();
    }
}

void PanelColumn::detachFlyoutForDrag(PanelGroup* group)
{
    if (!flyoutGroup_ || flyoutGroup_ != group) {
        return;
    }
    flyoutGroup_ = nullptr;
    flyoutName_.clear();
    flyoutGroupIndex_ = -1;
    flyoutFloat_ = nullptr;
    flyoutWasCollapsed_ = false;
    group->setFlyout(false);
    if (flyoutLayout_) {
        flyoutLayout_->removeWidget(group);
    }
    if (flyout_) {
        flyout_->hide();
    }
    setActiveIcon(QString());
}

QWidget* PanelColumn::iconFlyoutResizeGripForTest() const
{
    return flyout_ ? flyout_->resizeGripForTest() : nullptr;
}

void PanelColumn::restoreFlyoutGroup()
{
    if (!flyoutGroup_ || restoringFlyout_) {
        return;
    }
    restoringFlyout_ = true;
    PanelGroup* group = flyoutGroup_;
    const int index = flyoutGroupIndex_;
    PanelFloat* originFloat = flyoutFloat_;
    const bool wasCollapsed = flyoutWasCollapsed_;
    flyoutGroup_ = nullptr;
    flyoutName_.clear();
    flyoutGroupIndex_ = -1;
    flyoutFloat_ = nullptr;
    flyoutWasCollapsed_ = false;
    group->setFlyout(false);
    if (group->parentWidget() != flyout_) {
        // A drag rehomed the group (e.g. tore it off) before the flyout hid;
        // leave it where it went rather than stealing it back.
        restoringFlyout_ = false;
        setActiveIcon(QString());
        return;
    }
    if (flyoutLayout_) {
        flyoutLayout_->removeWidget(group);
    }
    if (originFloat) {
        // Phase 7: the compact icon opened this flyout, so the group goes back
        // into its overlay (not the splitter), re-collapsed, exactly once.
        group->setParent(nullptr);
        originFloat->setContent(group);
        group->setVisible(true);
        group->setFloating(true);
        originFloat->show();
        originFloat->raise();
        if (wasCollapsed) {
            group->setCollapsedToIcons(true);
        }
        originFloat->syncToContent();
        wireGroup(group);
        restoringFlyout_ = false;
        setActiveIcon(QString());
        maybeRemoveSelf();
        return;
    }
    // The group stayed in `groups_` while popped; only its widget parent
    // changed. Put it back at its remembered splitter slot exactly once.
    group->setParent(nullptr);
    splitter_->insertWidget(qBound(0, index, splitter_->count()), group);
    splitter_->setStretchFactor(qBound(0, index, splitter_->count()), 1);
    group->setFloating(false);
    group->setVisible(!group->visibleTitles().isEmpty());
    wireGroup(group);
    // A docked compact group was expanded only for the flyout; put the icon row
    // back on close.
    if (wasCollapsed) {
        group->setCollapsedToIcons(true);
    }
    restoringFlyout_ = false;
    setActiveIcon(QString());
    // `Auto-Collapse Iconic Panels`: once an icon flyout closes, return the
    // column to the iconic strip rather than leaving it expanded.
    if (autoCollapseIconic_ && !railMode_) {
        setRailMode(true);
    }
    maybeRemoveSelf();
}

QToolButton* PanelColumn::stripButtonFor(const QString& objectName) const
{
    if (!iconStrip_) {
        return nullptr;
    }
    return iconStrip_
        ->findChild<QToolButton*>(QStringLiteral("panelIcon_") + objectName);
}

} // namespace pictura
