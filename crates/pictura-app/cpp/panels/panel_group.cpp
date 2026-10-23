#include "panel_group.h"

#include "fonts.h"
#include "icons.h"

#include <QtCore/QEvent>
#include <QtGui/QContextMenuEvent>
#include <QtGui/QMouseEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QBoxLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSizePolicy>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <utility>

namespace pictura {

namespace {
constexpr int kIconButtonSize = 34;
constexpr int kIconPixmapSize = 24;
constexpr int kHeaderButtonSize = 18;
constexpr int kHeaderGripWidth = 16;
constexpr int kHeaderCornerRightMargin = 2;
// M47: matches the docked strip's `kCompactGripHeight`, so the collapsed float
// row and a strip group box share the same grip band.
constexpr int kIconGripHeight = 10;
} // namespace

PanelGroup::PanelGroup(QWidget* parent)
    : QWidget(parent)
{
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(0);

    // M47: the float-only top bar. Hidden while docked; `setFloating` shows it
    // when the group is hosted in a `PanelFloat`.
    floatHeader_ = new QWidget(this);
    floatHeader_->setObjectName(QStringLiteral("panelFloatHeader"));
    floatHeader_->setAttribute(Qt::WA_StyledBackground, true);
    // M47: the bar is a fixed-height strip; extra overlay height belongs to the
    // group body, not to padding around the toggle/close controls.
    floatHeader_->setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Fixed);
    auto* floatLayout = new QHBoxLayout(floatHeader_);
    floatLayout->setContentsMargins(2, 2, 2, 2);
    floatLayout->setSpacing(2);
    floatLayout->addStretch(1);
    floatToggle_ = new QToolButton(floatHeader_);
    floatToggle_->setObjectName(QStringLiteral("panelFloatToggle"));
    floatToggle_->setAutoRaise(true);
    floatToggle_->setFixedSize(18, 18);
    connect(floatToggle_, &QToolButton::clicked, this,
            [this]() { setCollapsedToIcons(!collapsedToIcons_); });
    floatLayout->addWidget(floatToggle_);
    floatCloseButton_ = new QToolButton(floatHeader_);
    floatCloseButton_->setObjectName(QStringLiteral("panelFloatClose"));
    floatCloseButton_->setAutoRaise(true);
    floatCloseButton_->setFixedSize(kHeaderButtonSize, kHeaderButtonSize);
    floatCloseButton_->setToolTip(tr("Close"));
    floatCloseButton_->setIcon(icon(QStringLiteral("panel.close")));
    floatCloseButton_->setVisible(false);
    floatLayout->addWidget(floatCloseButton_);
    floatHeader_->installEventFilter(this);
    floatHeader_->setVisible(false);
    layout->addWidget(floatHeader_);
    updateFloatToggle();

    tabs_ = new QTabWidget(this);
    tabs_->setObjectName(QStringLiteral("panelGroupTabs"));
    tabs_->setTabPosition(QTabWidget::North);
    tabs_->setDocumentMode(true);
    tabs_->tabBar()->setDrawBase(false);
    tabs_->tabBar()->installEventFilter(this);
    // M43: name the panel-group tab bar so the scoped theme can make the active
    // tab use the pane `${surface}` colour without touching the document tabs. The
    // tab bar elides instead of forcing width, and does not expand, so the
    // corner `▾` button keeps its place at the column minimum width.
    tabs_->tabBar()->setObjectName(QStringLiteral("panelTabBar"));
    tabs_->tabBar()->setElideMode(Qt::ElideRight);
    tabs_->tabBar()->setExpanding(false);
    // M47: overflow squeezes/elides the tabs instead of showing scroll arrows.
    tabs_->tabBar()->setUsesScrollButtons(false);
    // The theme draws the header's top rule itself; Qt's base frame would
    // break it under the current tab.
    tabs_->tabBar()->setDrawBase(false);
    // The theme paints the tab label two pixels under the app default, but
    // QTabBar computes its elision from the bar's own font; match the painted
    // size and weight so a label only elides when it genuinely does not fit.
    applyTabBarFont(tabs_->tabBar());
    layout->addWidget(tabs_);

    // M47: a lowered band behind the tab bar and corner paints the header strip
    // across the whole group width, including the slice above the corner button
    // that the tab bar does not reach.
    headerBand_ = new QWidget(tabs_);
    headerBand_->setObjectName(QStringLiteral("panelHeaderBand"));
    headerBand_->setAttribute(Qt::WA_StyledBackground, true);
    headerBand_->lower();
    tabs_->installEventFilter(this);

    headerCorner_ = new QWidget(tabs_);
    headerCorner_->setObjectName(QStringLiteral("panelWidgetCorner"));
    headerCorner_->setAttribute(Qt::WA_StyledBackground, true);
    auto* cornerLayout = new QHBoxLayout(headerCorner_);
    cornerLayout->setContentsMargins(0, 0, kHeaderCornerRightMargin, 0);
    cornerLayout->setSpacing(0);

    // M47: a reserved blank drag grip, the first corner child so it sits
    // immediately right of the tab bar.
    headerGrip_ = new QWidget(headerCorner_);
    headerGrip_->setObjectName(QStringLiteral("panelGroupDragGrip"));
    headerGrip_->setFixedWidth(kHeaderGripWidth);
    headerGrip_->setFixedHeight(kHeaderButtonSize);
    headerGrip_->setCursor(Qt::SizeAllCursor);
    headerGrip_->setToolTip(tr("Drag to move this panel group"));
    headerGrip_->setAttribute(Qt::WA_StyledBackground, true);
    headerGrip_->installEventFilter(this);
    cornerLayout->addWidget(headerGrip_, 0, Qt::AlignVCenter);

    // The flyout-only close control sits immediately left of the per-widget
    // menu button. It is hidden in docked and floating presentations.
    flyoutCloseButton_ = new QToolButton(headerCorner_);
    flyoutCloseButton_->setObjectName(QStringLiteral("panelFlyoutClose"));
    flyoutCloseButton_->setIcon(icon(QStringLiteral("panel.columnsTwo")));
    flyoutCloseButton_->setIconSize(QSize(14, 14));
    flyoutCloseButton_->setFixedSize(kHeaderButtonSize, kHeaderButtonSize);
    flyoutCloseButton_->setAutoRaise(true);
    flyoutCloseButton_->setToolTip(tr("Close Flyout"));
    flyoutCloseButton_->setVisible(false);
    connect(flyoutCloseButton_, &QToolButton::clicked, this,
            [this]() { emit flyoutCloseRequested(); });
    cornerLayout->addWidget(flyoutCloseButton_, 0, Qt::AlignVCenter);

    headerButton_ = new QToolButton(headerCorner_);
    headerButton_->setObjectName(QStringLiteral("panelWidgetMenu"));
    headerButton_->setIcon(icon(QStringLiteral("panel.menu")));
    // The glyph runs smaller than the button so it reads as a quiet affordance
    // rather than a filled control.
    headerButton_->setIconSize(QSize(13, 13));
    headerButton_->setAutoRaise(true);
    headerButton_->setPopupMode(QToolButton::InstantPopup);
    headerButton_->setFixedSize(kHeaderButtonSize, kHeaderButtonSize);
    headerButton_->setVisible(false);
    cornerLayout->addWidget(headerButton_, 0, Qt::AlignVCenter);

    tabs_->setCornerWidget(headerCorner_, Qt::TopRightCorner);
    connect(tabs_, &QTabWidget::currentChanged, this, [this]() { updateHeaderMenu(); });

    // M47: the collapsed row mirrors the docked strip's per-group box: an
    // `panelIconGroup` container with a `panelIconGroupGrip` divider over the
    // 34 px icon buttons, so a collapsed float and a strip group read alike.
    iconRow_ = new QWidget(this);
    iconRow_->setObjectName(QStringLiteral("panelIconGroup"));
    iconRow_->setAttribute(Qt::WA_StyledBackground, true);
    auto* iconRowOuterLayout = new QVBoxLayout(iconRow_);
    iconRowOuterLayout->setContentsMargins(1, 1, 1, 1);
    iconRowOuterLayout->setSpacing(1);
    auto* iconGrip = new QLabel(iconRow_);
    iconGrip->setObjectName(QStringLiteral("panelIconGroupGrip"));
    iconGrip->setAlignment(Qt::AlignCenter);
    iconGrip->setText(QStringLiteral("\u2022\u2022\u2022"));
    iconGrip->setFixedHeight(kIconGripHeight);
    iconGrip->setCursor(Qt::SizeAllCursor);
    iconGrip->setToolTip(tr("Drag to move this panel group"));
    iconGrip->setAttribute(Qt::WA_StyledBackground, true);
    // Phase 7: the collapsed float row's grip drags the whole group through the
    // same `groupDragStarted` path the tab-bar grip and float header use.
    iconGrip->installEventFilter(this);
    iconRowOuterLayout->addWidget(iconGrip);
    auto* iconRowInner = new QWidget(iconRow_);
    iconRowInner->setObjectName(QStringLiteral("panelIconRow"));
    iconRowLayout_ = new QVBoxLayout(iconRowInner);
    iconRowLayout_->setContentsMargins(0, 0, 0, 0);
    iconRowLayout_->setSpacing(4);
    iconRowLayout_->addStretch(1);
    iconRowOuterLayout->addWidget(iconRowInner);
    iconRow_->setVisible(false);
    layout->addWidget(iconRow_);
}

void PanelGroup::addPanel(QWidget* panel, const QString& title, const QIcon& icon)
{
    if (!panel) {
        return;
    }
    panelIcons_.insert(panel->objectName(), icon);
    const int index = tabs_->addTab(panel, QIcon(), title);
    tabs_->setTabToolTip(index, title);
    if (tabs_->count() == 1) {
        setObjectName(QStringLiteral("panelGroup_") + panel->objectName());
    }
    if (collapsedToIcons_) {
        rebuildIconRow();
    }
    updateHeaderMenu();
}

QStringList PanelGroup::titles() const
{
    QStringList out;
    for (int i = 0; i < tabs_->count(); ++i) {
        out << tabs_->tabText(i);
    }
    return out;
}

void PanelGroup::setPanelOrder(const QStringList& order)
{
    const int count = tabs_->count();
    QList<QWidget*> panels;
    QStringList titles;
    for (int i = 0; i < count; ++i) {
        panels << tabs_->widget(i);
        titles << tabs_->tabText(i);
    }
    QList<int> sequence;
    for (const QString& name : order) {
        for (int i = 0; i < panels.size(); ++i) {
            if (panels.at(i) && panels.at(i)->objectName() == name && !sequence.contains(i)) {
                sequence << i;
                break;
            }
        }
    }
    for (int i = 0; i < panels.size(); ++i) {
        if (!sequence.contains(i)) {
            sequence << i;
        }
    }
    // QTabBar::moveTab reorders the tab bar but not QTabWidget's page stack, so
    // re-insert the pages to keep `panels()` and `titles()` in the same order.
    for (int i = tabs_->count() - 1; i >= 0; --i) {
        tabs_->removeTab(i);
    }
    for (int index : sequence) {
        tabs_->addTab(panels.at(index), QIcon(), titles.at(index));
    }
    if (collapsedToIcons_) {
        rebuildIconRow();
    }
}

bool PanelGroup::isPanelVisible(const QString& objectName) const
{
    for (int i = 0; i < tabs_->count(); ++i) {
        QWidget* panel = tabs_->widget(i);
        if (panel && panel->objectName() == objectName) {
            return tabs_->isTabVisible(i);
        }
    }
    return false;
}

QStringList PanelGroup::visibleTitles() const
{
    QStringList out;
    for (int i = 0; i < tabs_->count(); ++i) {
        if (tabs_->isTabVisible(i)) {
            out << tabs_->tabText(i);
        }
    }
    return out;
}

QList<QWidget*> PanelGroup::panels() const
{
    QList<QWidget*> out;
    for (int i = 0; i < tabs_->count(); ++i) {
        if (QWidget* panel = tabs_->widget(i)) {
            out << panel;
        }
    }
    return out;
}

QList<QWidget*> PanelGroup::visiblePanels() const
{
    QList<QWidget*> out;
    for (int i = 0; i < tabs_->count(); ++i) {
        if (tabs_->isTabVisible(i)) {
            if (QWidget* panel = tabs_->widget(i)) {
                out << panel;
            }
        }
    }
    return out;
}

bool PanelGroup::containsPanel(const QString& objectName) const
{
    for (int i = 0; i < tabs_->count(); ++i) {
        QWidget* panel = tabs_->widget(i);
        if (panel && panel->objectName() == objectName) {
            return true;
        }
    }
    return false;
}

bool PanelGroup::setPanelVisible(const QString& objectName, bool visible)
{
    for (int i = 0; i < tabs_->count(); ++i) {
        QWidget* panel = tabs_->widget(i);
        if (panel && panel->objectName() == objectName) {
            tabs_->setTabVisible(i, visible);
            if (visible) {
                tabs_->setCurrentIndex(i);
            }
            setVisible(!visibleTitles().isEmpty());
            if (collapsedToIcons_) {
                rebuildIconRow();
            }
            return true;
        }
    }
    return false;
}

QString PanelGroup::currentPanelName() const
{
    QWidget* panel = tabs_->currentWidget();
    return panel ? panel->objectName() : QString();
}

bool PanelGroup::setCurrentPanel(const QString& objectName)
{
    for (int i = 0; i < tabs_->count(); ++i) {
        QWidget* panel = tabs_->widget(i);
        if (panel && panel->objectName() == objectName) {
            tabs_->setCurrentIndex(i);
            return true;
        }
    }
    return false;
}

void PanelGroup::setCurrentToFirstVisible()
{
    for (int i = 0; i < tabs_->count(); ++i) {
        if (tabs_->isTabVisible(i)) {
            tabs_->setCurrentIndex(i);
            return;
        }
    }
}

void PanelGroup::setPanelMenuHook(const QString& panelName, PanelMenuHook hook)
{
    if (hook) {
        panelMenuHooks_.insert(panelName, std::move(hook));
    } else {
        panelMenuHooks_.remove(panelName);
    }
}

QString PanelGroup::titleForPanel(const QString& objectName) const
{
    for (int i = 0; i < tabs_->count(); ++i) {
        QWidget* panel = tabs_->widget(i);
        if (panel && panel->objectName() == objectName) {
            return tabs_->tabText(i);
        }
    }
    return QString();
}

QIcon PanelGroup::iconForPanel(const QString& objectName) const
{
    for (int i = 0; i < tabs_->count(); ++i) {
        QWidget* panel = tabs_->widget(i);
        if (panel && panel->objectName() == objectName) {
            return panelIcons_.value(objectName);
        }
    }
    return QIcon();
}

QWidget* PanelGroup::takePanel(const QString& objectName, QString* title, QIcon* icon, int* index)
{
    for (int i = 0; i < tabs_->count(); ++i) {
        QWidget* panel = tabs_->widget(i);
        if (panel && panel->objectName() == objectName) {
            if (title) {
                *title = tabs_->tabText(i);
            }
            if (icon) {
                *icon = panelIcons_.value(objectName);
            }
            if (index) {
                *index = i;
            }
            tabs_->removeTab(i);
            if (collapsedToIcons_) {
                rebuildIconRow();
            }
            return panel;
        }
    }
    return nullptr;
}

void PanelGroup::insertPanel(QWidget* panel, const QString& title, const QIcon& icon, int index)
{
    if (!panel) {
        return;
    }
    index = qBound(0, index, tabs_->count());
    panelIcons_.insert(panel->objectName(), icon);
    const int actual = tabs_->insertTab(index, panel, QIcon(), title);
    tabs_->setTabToolTip(actual, title);
    tabs_->setCurrentIndex(actual);
    if (collapsedToIcons_) {
        rebuildIconRow();
    }
}

QTabBar* PanelGroup::tabBar() const
{
    return tabs_ ? tabs_->tabBar() : nullptr;
}

int PanelGroup::indexOfPanel(const QString& objectName) const
{
    for (int i = 0; i < tabs_->count(); ++i) {
        QWidget* panel = tabs_->widget(i);
        if (panel && panel->objectName() == objectName) {
            return i;
        }
    }
    return -1;
}

QRect PanelGroup::tabBarGlobalRect() const
{
    QTabBar* bar = tabBar();
    if (!bar) {
        return QRect();
    }
    return QRect(bar->mapToGlobal(QPoint(0, 0)), bar->size());
}

int PanelGroup::tabInsertionIndexAt(const QPoint& globalPos) const
{
    QTabBar* bar = tabBar();
    if (!bar) {
        return -1;
    }
    const QPoint pos = bar->mapFromGlobal(globalPos);
    if (!bar->rect().contains(pos)) {
        return -1;
    }
    // Skip hidden tabs (whose rects are empty), matching `tabInsertionX`, so the
    // drawn line and the committed insert agree for a group with hidden tabs.
    for (int i = 0; i < bar->count(); ++i) {
        if (!bar->isTabVisible(i)) {
            continue;
        }
        if (pos.x() < bar->tabRect(i).center().x()) {
            return i;
        }
    }
    return bar->count();
}

int PanelGroup::tabInsertionX(int index) const
{
    QTabBar* bar = tabBar();
    if (!bar || bar->count() == 0) {
        return 0;
    }
    const int count = bar->count();
    index = qBound(0, index, count);
    // M47: with scroll buttons off a squeezed tab can be clipped past the bar's
    // right edge, so clamp every candidate point inside the bar; otherwise a
    // drop at the end of a full tab bar resolves to the group body instead.
    const int maxX = qMax(0, bar->rect().right());
    // ponytail: hidden QTabBar tabs have empty rects, so scan for the first
    // visible tab instead of trusting tabRect(index).
    for (int i = index; i < count; ++i) {
        const QRect rect = bar->tabRect(i);
        if (rect.isValid() && !rect.isEmpty()) {
            return qBound(0, rect.left(), maxX);
        }
    }
    for (int i = qMin(index, count) - 1; i >= 0; --i) {
        const QRect rect = bar->tabRect(i);
        if (rect.isValid() && !rect.isEmpty()) {
            return qBound(0, rect.right() + 1, maxX);
        }
    }
    return maxX;
}

void PanelGroup::setMinimized(bool minimized)
{
    if (minimized_ == minimized) {
        return;
    }
    minimized_ = minimized;
    applyMinimize();
}

void PanelGroup::applyMinimize()
{
    if (minimized_) {
        if (collapsedToIcons_) {
            setCollapsedToIcons(false);
        }
        savedMaxHeight_ = tabs_->maximumHeight();
        savedGroupMaxHeight_ = maximumHeight();
        const int barHeight = tabs_->tabBar() ? tabs_->tabBar()->sizeHint().height() : 24;
        savedTabsMinHeight_ = tabs_->minimumHeight();
        savedMinHeight_ = minimumHeight();
        tabs_->setMinimumHeight(barHeight);
        setMinimumHeight(barHeight);
        // M45 W5: clamp the group itself, not only its tab widget, so the
        // splitter gives it just the tab-bar height.
        tabs_->setMaximumHeight(barHeight);
        setMaximumHeight(barHeight);
        setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Fixed);
    } else {
        tabs_->setMinimumHeight(savedTabsMinHeight_);
        setMinimumHeight(savedMinHeight_);
        tabs_->setMaximumHeight(savedMaxHeight_);
        setMaximumHeight(savedGroupMaxHeight_);
        setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Expanding);
    }
    updateGeometry();
}

void PanelGroup::setCollapsedToIcons(bool collapsed)
{
    if (collapsedToIcons_ == collapsed) {
        return;
    }
    collapsedToIcons_ = collapsed;
    if (collapsed) {
        if (minimized_) {
            minimized_ = false;
            applyMinimize();
        }
        rebuildIconRow();
    }
    tabs_->setVisible(!collapsed);
    iconRow_->setVisible(collapsed);
    updateFloatToggle();
    emit collapsedToIconsChanged(collapsed);
}

void PanelGroup::updateFloatToggle()
{
    if (!floatToggle_) {
        return;
    }
    // M47: mirror `PanelColumn::updateColumnToggle` — show the icon for the
    // action performed: collapse-to-icons when expanded, expand when collapsed.
    const QIcon target = icon(collapsedToIcons_ ? QStringLiteral("panel.columnsOne")
                                                : QStringLiteral("panel.columnsTwo"));
    if (!target.isNull()) {
        floatToggle_->setIcon(target);
        floatToggle_->setIconSize(QSize(16, 16));
        floatToggle_->setText(QString());
    } else {
        floatToggle_->setIcon(QIcon());
        floatToggle_->setText(collapsedToIcons_ ? QStringLiteral("\u00ab")
                                                : QStringLiteral("\u00bb"));
    }
    floatToggle_->setToolTip(collapsedToIcons_ ? tr("Expand panels")
                                               : tr("Collapse panels to icons"));
}
void PanelGroup::updateHeaderBand()
{
    if (!headerBand_ || !tabs_ || !tabs_->tabBar()) {
        return;
    }
    const int barHeight = tabs_->tabBar()->height();
    // QTabWidget bottom-aligns the corner to the tab bar, which drops the menu
    // button below the tab text. Give the corner symmetric padding so its own
    // height matches the tab bar and the centred layout lifts the button level
    // with the tabs.
    if (headerCorner_ && headerCorner_->layout() && barHeight > 0) {
        const int pad = qMax(0, (barHeight - kHeaderButtonSize) / 2);
        headerCorner_->layout()->setContentsMargins(0, pad, kHeaderCornerRightMargin, pad);
    }
    const int bandHeight =
        qMax(barHeight, headerCorner_ ? headerCorner_->height() : 0);
    headerBand_->setGeometry(0, 0, tabs_->width(), bandHeight);
    headerBand_->lower();
}

void PanelGroup::rebuildIconRow()
{
    while (QLayoutItem* item = iconRowLayout_->takeAt(0)) {
        if (QWidget* widget = item->widget()) {
            delete widget;
        }
        delete item;
    }
    for (int i = 0; i < tabs_->count(); ++i) {
        if (!tabs_->isTabVisible(i)) {
            continue;
        }
        QWidget* panel = tabs_->widget(i);
        if (!panel) {
            continue;
        }
        iconRowLayout_->addWidget(
            makeIconButton(panelIcons_.value(panel->objectName()), tabs_->tabText(i),
                           panel->objectName()));
    }
    iconRowLayout_->addStretch(1);
}

QToolButton* PanelGroup::makeIconButton(const QIcon& icon, const QString& title,
                                        const QString& objectName)
{
    auto* button = new QToolButton(iconRow_);
    button->setObjectName(QStringLiteral("panelGroupIcon_") + objectName);
    if (!icon.isNull()) {
        button->setIcon(icon);
        button->setIconSize(QSize(kIconPixmapSize, kIconPixmapSize));
    } else {
        button->setText(title.left(1));
    }
    button->setToolTip(title);
    button->setAutoRaise(true);
    button->setFixedSize(kIconButtonSize, kIconButtonSize);
    connect(button, &QToolButton::clicked, this, [this, objectName, button]() {
        emit panelActivated(objectName, button->mapToGlobal(QPoint(button->width(), 0)));
    });
    // A floating icon is also a drag source: the same tab-drag grammar the
    // docked strip's buttons use, so a panel can be torn out of a floating group.
    button->installEventFilter(this);
    return button;
}

bool PanelGroup::eventFilter(QObject* watched, QEvent* event)
{
    if ((watched == tabs_ || (tabs_ && watched == tabs_->tabBar()))
        && event->type() == QEvent::Resize) {
        updateHeaderBand();
    }
    // M47: the corner drag grip and the float header both drive a whole-group
    // drag through the same signals as the empty tab-bar path. Each keeps its
    // own state so the gestures cannot clobber one another.
    auto groupGesture = [this](QEvent* e, bool& pressPending, bool& dragging,
                               QPoint& pressGlobal) -> bool {
        const QEvent::Type type = e->type();
        if (type == QEvent::MouseButtonPress) {
            auto* mouse = static_cast<QMouseEvent*>(e);
            if (mouse->button() == Qt::LeftButton) {
                pressPending = true;
                dragging = false;
                pressGlobal = mouse->globalPosition().toPoint();
            }
        } else if (type == QEvent::MouseMove) {
            auto* mouse = static_cast<QMouseEvent*>(e);
            const QPoint globalPos = mouse->globalPosition().toPoint();
            if (pressPending && !dragging
                && (globalPos - pressGlobal).manhattanLength()
                       >= QApplication::startDragDistance()) {
                pressPending = false;
                dragging = true;
                emit groupDragStarted(globalPos);
            }
            if (dragging) {
                emit dragMoved(globalPos);
                return true;
            }
        } else if (type == QEvent::MouseButtonRelease) {
            pressPending = false;
            if (dragging) {
                dragging = false;
                auto* mouse = static_cast<QMouseEvent*>(e);
                emit dragFinished(mouse->globalPosition().toPoint());
                return true;
            }
        }
        return false;
    };
    if (watched == headerGrip_) {
        if (groupGesture(event, gripPressPending_, gripDragging_, gripPressGlobal_)) {
            return true;
        }
    }
    if (watched == floatHeader_) {
        if (groupGesture(event, floatPressPending_, floatDragging_, floatPressGlobal_)) {
            return true;
        }
    }
    // Phase 7: the collapsed float row's grip drags the whole group through the
    // same gesture/emission path as the float header.
    if (auto* widget = qobject_cast<QWidget*>(watched);
        widget && iconRow_ && widget != floatHeader_ && widget != headerGrip_
        && widget->objectName() == QStringLiteral("panelIconGroupGrip")
        && widget->parentWidget() == iconRow_) {
        if (groupGesture(event, floatPressPending_, floatDragging_, floatPressGlobal_)) {
            return true;
        }
    }
    // Phase 7: an icon in the collapsed row drags its panel through the same
    // `tabDragStarted`/`dragMoved`/`dragFinished` grammar the docked strip uses.
    // A press+release below the threshold still emits `panelActivated` (clicked).
    if (auto* iconButton = qobject_cast<QToolButton*>(watched);
        iconButton && iconButton->objectName().startsWith(QStringLiteral("panelGroupIcon_"))) {
        const QEvent::Type type = event->type();
        if (type == QEvent::MouseButtonPress) {
            auto* mouse = static_cast<QMouseEvent*>(event);
            if (mouse->button() == Qt::LeftButton) {
                iconPressPending_ = true;
                iconDragging_ = false;
                iconDragName_ = iconButton->objectName().mid(
                    QStringLiteral("panelGroupIcon_").size());
                iconPressGlobal_ = mouse->globalPosition().toPoint();
            }
        } else if (type == QEvent::MouseMove) {
            auto* mouse = static_cast<QMouseEvent*>(event);
            const QPoint globalPos = mouse->globalPosition().toPoint();
            if (iconPressPending_ && !iconDragging_
                && (globalPos - iconPressGlobal_).manhattanLength()
                       >= QApplication::startDragDistance()) {
                iconPressPending_ = false;
                iconDragging_ = true;
                emit tabDragStarted(iconDragName_, globalPos);
            }
            if (iconDragging_) {
                emit dragMoved(globalPos);
                return true;
            }
        } else if (type == QEvent::MouseButtonRelease) {
            iconPressPending_ = false;
            if (iconDragging_) {
                iconDragging_ = false;
                auto* mouse = static_cast<QMouseEvent*>(event);
                emit dragFinished(mouse->globalPosition().toPoint());
                return true;
            }
            iconDragName_.clear();
        }
    }
    if (tabs_ && watched == tabs_->tabBar()) {
        const QEvent::Type type = event->type();
        if (type == QEvent::ContextMenu) {
            auto* menuEvent = static_cast<QContextMenuEvent*>(event);
            emit tabContextMenuRequested(menuEvent->globalPos());
            return true;
        }
        if (type == QEvent::MouseButtonPress) {
            auto* mouseEvent = static_cast<QMouseEvent*>(event);
            if (mouseEvent->button() == Qt::LeftButton) {
                pressPending_ = true;
                dragging_ = false;
                pressGlobal_ = mouseEvent->globalPosition().toPoint();
                const int index = tabs_->tabBar()->tabAt(mouseEvent->position().toPoint());
                QWidget* panel = index >= 0 ? tabs_->widget(index) : nullptr;
                pressedPanel_ = panel ? panel->objectName() : QString();
            }
        } else if (type == QEvent::MouseMove) {
            auto* mouseEvent = static_cast<QMouseEvent*>(event);
            const QPoint globalPos = mouseEvent->globalPosition().toPoint();
            if (pressPending_ && !dragging_
                && (globalPos - pressGlobal_).manhattanLength()
                       >= QApplication::startDragDistance()) {
                pressPending_ = false;
                dragging_ = true;
                if (pressedPanel_.isEmpty()) {
                    emit groupDragStarted(globalPos);
                } else {
                    emit tabDragStarted(pressedPanel_, globalPos);
                }
            }
            if (dragging_) {
                emit dragMoved(globalPos);
                return true;
            }
        } else if (type == QEvent::MouseButtonRelease) {
            pressPending_ = false;
            if (dragging_) {
                dragging_ = false;
                auto* mouseEvent = static_cast<QMouseEvent*>(event);
                emit dragFinished(mouseEvent->globalPosition().toPoint());
                pressedPanel_.clear();
                return true;
            }
            pressedPanel_.clear();
        }
    }
    return QWidget::eventFilter(watched, event);
}

void PanelGroup::setFloating(bool on)
{
    if (floatCloseButton_) {
        floatCloseButton_->setVisible(on);
    }
    if (floatHeader_) {
        floatHeader_->setVisible(on);
    }
    if (on && flyoutCloseButton_) {
        flyoutCloseButton_->setVisible(false);
    }
    updateFloatToggle();
    updateHeaderMenu();
}

void PanelGroup::setFlyout(bool on)
{
    if (flyoutCloseButton_) {
        flyoutCloseButton_->setVisible(on);
    }
    if (on && floatHeader_) {
        floatHeader_->setVisible(false);
        if (floatCloseButton_) {
            floatCloseButton_->setVisible(false);
        }
    }
    updateHeaderMenu();
    updateHeaderBand();
}

} // namespace pictura

#include "moc_panel_group.cpp"
