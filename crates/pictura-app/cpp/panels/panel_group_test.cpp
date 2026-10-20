#include "panel_group.h"

#include <QtGui/QAction>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLayout>
#include <QtWidgets/QMenu>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QToolButton>

namespace pictura {

int PanelGroup::headerCornerWidthForTest() const
{
    // The reserved grip, the flyout-only close control when visible, and the
    // per-widget menu button; the corner keeps a right margin.
    int width = 0;
    if (headerGrip_) {
        width += headerGrip_->width();
    }
    if (flyoutCloseButton_ && flyoutCloseButton_->isVisible()) {
        width += flyoutCloseButton_->width();
    }
    if (headerButton_) {
        width += headerButton_->width();
    }
    width += headerCornerRightMarginForTest();
    return width;
}

int PanelGroup::headerCornerRightMarginForTest() const
{
    if (!headerCorner_ || !headerCorner_->layout()) {
        return 0;
    }
    return headerCorner_->layout()->contentsMargins().right();
}

bool PanelGroup::tabUsesScrollButtonsForTest() const
{
    return tabBar() && tabBar()->usesScrollButtons();
}

bool PanelGroup::floatHeaderVisibleForTest() const
{
    return floatHeader_ && floatHeader_->isVisible();
}

QPoint PanelGroup::tabInsertionGlobalPointForTest(int index) const
{
    QTabBar* bar = tabBar();
    if (!bar) {
        return QPoint();
    }
    return bar->mapToGlobal(QPoint(tabInsertionX(index), bar->height() / 2));
}

int PanelGroup::tabPositionForTest() const
{
    return static_cast<int>(tabs_->tabPosition());
}

int PanelGroup::titleCountForTest() const
{
    return tabs_->count();
}

QStringList PanelGroup::titleTextsForTest() const
{
    return titles();
}

QIcon PanelGroup::titleIconForTest(const QString& title) const
{
    // The icon is no longer on the normal tab; read the retained store.
    for (int i = 0; i < tabs_->count(); ++i) {
        if (tabs_->tabText(i) == title) {
            QWidget* panel = tabs_->widget(i);
            return panel ? panelIcons_.value(panel->objectName()) : QIcon();
        }
    }
    return QIcon();
}

bool PanelGroup::tabHasIconForTest(const QString& title) const
{
    for (int i = 0; i < tabs_->count(); ++i) {
        if (tabs_->tabText(i) == title) {
            return !tabs_->tabIcon(i).isNull();
        }
    }
    return false;
}

QIcon PanelGroup::iconicIconForTest(const QString& objectName) const
{
    if (!iconRow_) {
        return QIcon();
    }
    QToolButton* button =
        iconRow_->findChild<QToolButton*>(QStringLiteral("panelGroupIcon_") + objectName);
    return button ? button->icon() : QIcon();
}

bool PanelGroup::groupLabelForTest() const
{
    return findChild<QLabel*>(QStringLiteral("panelGroupLabel")) != nullptr;
}

bool PanelGroup::contentHiddenForTest() const
{
    return tabs_->maximumHeight() < QWIDGETSIZE_MAX;
}

bool PanelGroup::tabBarVisibleForTest() const
{
    return tabs_->tabBar() && tabs_->tabBar()->isVisible();
}

int PanelGroup::firstVisibleTabIndexForTest() const
{
    for (int i = 0; i < tabs_->count(); ++i) {
        if (tabs_->isTabVisible(i)) {
            return i;
        }
    }
    return -1;
}

int PanelGroup::currentTabIndexForTest() const
{
    return tabs_->currentIndex();
}

bool PanelGroup::headerMenuAtRightForTest() const
{
    if (!headerButton_ || !headerButton_->isVisible()
        || tabs_->cornerWidget(Qt::TopRightCorner) != headerCorner_
        || !tabs_->isAncestorOf(headerButton_)) {
        return false;
    }
    QTabBar* bar = tabs_->tabBar();
    if (!bar) {
        return false;
    }
    // The corner widget sits immediately to the right of the tab bar.
    const QPoint topLeft = headerButton_->mapTo(tabs_, QPoint(0, 0));
    return topLeft.x() >= bar->width() - 2
           && topLeft.x() + headerButton_->width() <= tabs_->width();
}

QStringList PanelGroup::panelMenuTextsForTest() const
{
    QStringList texts;
    if (!headerMenu_) {
        return texts;
    }
    for (QAction* action : headerMenu_->actions()) {
        if (!action->isSeparator()) {
            texts.push_back(action->text());
        }
    }
    return texts;
}

static QAction* findMenuAction(QMenu* menu, const QString& text)
{
    if (!menu) {
        return nullptr;
    }
    for (QAction* action : menu->actions()) {
        if (action->text() == text) {
            return action;
        }
        if (QMenu* child = action->menu()) {
            for (QAction* sub : child->actions()) {
                if (sub->text() == text) {
                    return sub;
                }
            }
        }
    }
    return nullptr;
}

bool PanelGroup::panelMenuEnabledForTest(const QString& text) const
{
    QAction* action = findMenuAction(headerMenu_, text);
    return action && action->isEnabled();
}

QString PanelGroup::panelMenuToolTipForTest(const QString& text) const
{
    QAction* action = findMenuAction(headerMenu_, text);
    return action ? action->toolTip() : QString();
}

bool PanelGroup::triggerPanelMenuForTest(const QString& text)
{
    QAction* action = findMenuAction(headerMenu_, text);
    if (!action || !action->isEnabled()) {
        return false;
    }
    action->trigger();
    return true;
}

} // namespace pictura
