#include "panel_column.h"

#include "panel_column_internal.h"

#include "panel_host.h"
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

// One source of truth for the tab-menu labels: `buildTabMenu` constructs the
// actions from this list and `tabMenuTextsFor` reports the same strings, so a
// rename cannot desync the menu from the test.
QStringList tabMenuTexts(bool minimized)
{
    return {
        PanelColumn::tr("Close"),
        PanelColumn::tr("Close Panel Group"),
        minimized ? PanelColumn::tr("Expand Panel") : PanelColumn::tr("Minimize"),
        PanelColumn::tr("Collapse to Icons"),
        PanelColumn::tr("Auto-Collapse Iconic Panels"),
        PanelColumn::tr("Auto-Show Hidden Panels"),
        PanelColumn::tr("Interface Options\u2026"),
    };
}

// The column-header menu labels: one source of truth for the menu and its test.
QStringList columnHeaderMenuTexts()
{
    return {
        PanelColumn::tr("Collapse to Icons"),
        PanelColumn::tr("Auto-Collapse Iconic Panels"),
        PanelColumn::tr("Auto-Show Hidden Panels"),
        PanelColumn::tr("Interface Options\u2026"),
    };
}

} // namespace

void PanelColumn::showColumnHeaderMenu(const QPoint& globalPos)
{
    QMenu* menu = buildColumnHeaderMenu();
    menu->setAttribute(Qt::WA_DeleteOnClose);
    menu->popup(globalPos);
}

QMenu* PanelColumn::buildColumnHeaderMenu()
{
    const QStringList labels = columnHeaderMenuTexts();
    auto* menu = new QMenu(this);
    menu->setObjectName(QStringLiteral("panelColumnHeaderMenu"));

    QAction* collapse = menu->addAction(labels[0]);
    collapse->setCheckable(true);
    collapse->setChecked(railMode_);
    connect(collapse, &QAction::triggered, this, [this](bool on) { setRailMode(on); });

    QAction* autoCollapse = menu->addAction(labels[1]);
    autoCollapse->setCheckable(true);
    autoCollapse->setChecked(autoCollapseIconic_);
    connect(autoCollapse, &QAction::triggered, this,
            [this](bool on) { setAutoCollapseIconic(on); });

    QAction* autoShow = menu->addAction(labels[2]);
    autoShow->setCheckable(true);
    autoShow->setChecked(autoShowHidden_);
    connect(autoShow, &QAction::triggered, this, [this](bool on) { setAutoShowHidden(on); });

    menu->addSeparator();

    QAction* options = menu->addAction(labels[3]);
    connect(options, &QAction::triggered, this, [this]() { emit interfaceOptionsRequested(); });

    return menu;
}

void PanelColumn::showTabMenu(PanelGroup* group, const QPoint& globalPos)
{
    menuGroup_ = group;
    QMenu* menu = buildTabMenu(group);
    menu->setAttribute(Qt::WA_DeleteOnClose);
    menu->popup(globalPos);
}

QMenu* PanelColumn::buildTabMenu(PanelGroup* group)
{
    auto* menu = new QMenu(this);
    menu->setObjectName(QStringLiteral("panelTabMenu"));

    const bool minimized = group && group->isMinimized();
    const QStringList labels = tabMenuTexts(minimized);

    QAction* close = menu->addAction(labels[0]);
    connect(close, &QAction::triggered, this, [this, group]() {
        if (!group) {
            return;
        }
        const QString name = group->currentPanelName();
        if (!name.isEmpty()) {
            showPanel(name, false);
        }
    });

    QAction* closeGroupAction = menu->addAction(labels[1]);
    connect(closeGroupAction, &QAction::triggered, this,
            [this, group]() { closeGroup(group); });

    QAction* minimize = menu->addAction(labels[2]);
    connect(minimize, &QAction::triggered, this, [this, group]() {
        group->setMinimized(!group->isMinimized());
        emit stateChanged();
    });

    QAction* collapse = menu->addAction(labels[3]);
    connect(collapse, &QAction::triggered, this, [this, group]() {
        group->setCollapsedToIcons(!group->isCollapsedToIcons());
        emit stateChanged();
    });

    menu->addSeparator();

    QAction* autoCollapse = menu->addAction(labels[4]);
    autoCollapse->setCheckable(true);
    autoCollapse->setChecked(autoCollapseIconic_);
    connect(autoCollapse, &QAction::triggered, this, [this](bool on) {
        setAutoCollapseIconic(on);
    });

    QAction* autoShow = menu->addAction(labels[5]);
    autoShow->setCheckable(true);
    autoShow->setChecked(autoShowHidden_);
    connect(autoShow, &QAction::triggered, this, [this](bool on) { setAutoShowHidden(on); });

    menu->addSeparator();

    QAction* options = menu->addAction(labels[6]);
    connect(options, &QAction::triggered, this, [this]() { emit interfaceOptionsRequested(); });

    return menu;
}

QStringList PanelColumn::tabMenuTextsFor(const PanelGroup* group) const
{
    // M45 W5: the minimize entry is a function of the group's state.
    return tabMenuTexts(group && group->isMinimized());
}

} // namespace pictura
