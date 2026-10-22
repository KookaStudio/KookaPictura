// Help > Search (Shift+Space): the overlay, what running or revealing an entry
// does, and the status-bar hint. Not a CS6 feature (#788); post-CS6 by owner
// decision.

#include "frame_includes.h"

#include "search_bar.h"
#include "search_index.h"

#include <QtWidgets/QComboBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>
#include <QtWidgets/QStatusBar>

namespace pictura {

namespace {

bool toolForName(const QString& name, ToolId* out)
{
    for (ToolId id : allToolIds()) {
        if (toolIdName(id) == name) {
            *out = id;
            return true;
        }
    }
    return false;
}

// Open the menu bar on `leaf`: its top-level menu, then each submenu, ending
// with the item active.
void revealMenuChain(QMenuBar* bar, QAction* leaf)
{
    QList<QAction*> chain;
    for (QAction* current = leaf; current;) {
        chain.prepend(current);
        QMenu* owner = nullptr;
        for (QObject* object : current->associatedObjects()) {
            if ((owner = qobject_cast<QMenu*>(object))) {
                break;
            }
        }
        current = owner ? owner->menuAction() : nullptr;
    }
    if (!bar || chain.size() < 2 || !chain.first()->menu()) {
        return;
    }
    bar->setActiveAction(chain.first());
    for (int i = 1; i < chain.size(); ++i) {
        if (QMenu* menu = chain.at(i - 1)->menu()) {
            menu->setActiveAction(chain.at(i));
        }
    }
}

void focusControl(QWidget* control)
{
    if (auto* combo = qobject_cast<QComboBox*>(control)) {
        combo->setFocus(Qt::ShortcutFocusReason);
        combo->showPopup();
        return;
    }
    auto* edit = qobject_cast<QLineEdit*>(control);
    if (!edit) {
        edit = control->findChild<QLineEdit*>();
    }
    if (edit) {
        edit->setFocus(Qt::ShortcutFocusReason);
        edit->selectAll();
        return;
    }
    control->setFocus(Qt::ShortcutFocusReason);
}

} // namespace

void PicturaMainWindow::registerSearchHandlers()
{
    registry_->setHandler(command_ids::HelpSearch, [this]() { openSearch(); });
}

void PicturaMainWindow::openSearch()
{
    if (!searchBar_) {
        searchBar_ = new SearchBar(this);
        connect(searchBar_, &SearchBar::runRequested, this, &PicturaMainWindow::runSearchEntry);
        connect(searchBar_, &SearchBar::revealRequested, this,
                &PicturaMainWindow::revealSearchEntry);
    }
    registry_->refresh();
    QWidget* host = imageView();
    if (!host) {
        host = tabs_ ? static_cast<QWidget*>(tabs_) : centralWidget();
    }
    const QRect rect(host->mapTo(this, QPoint(0, 0)), host->size());
    searchBar_->open(buildSearchIndex(*registry_), rect);
}

QWidget* PicturaMainWindow::showPanelForSearch(const QString& objectName)
{
    PanelColumn* owner = columnForPanel(objectName);
    if (!owner) {
        owner = panelColumn_;
    }
    if (owner) {
        owner->showPanel(objectName, true);
    }
    registry_->refresh();
    return findChild<QWidget*>(objectName);
}

QWidget* PicturaMainWindow::searchControlWidget(const SearchEntry& entry)
{
    if (ToolId tool; !entry.tool.isEmpty() && toolForName(entry.tool, &tool)) {
        setActiveTool(tool);
    }
    QWidget* container = entry.container == QLatin1String("optionsBar")
                             ? static_cast<QWidget*>(optionsBar_)
                             : showPanelForSearch(entry.container);
    if (!container) {
        return nullptr;
    }
    // Several tool pages share a control name; the shown page's is the one.
    const QList<QWidget*> found = container->findChildren<QWidget*>(entry.id);
    for (QWidget* widget : found) {
        if (widget->isVisibleTo(container)) {
            return widget;
        }
    }
    return found.value(0);
}

void PicturaMainWindow::runSearchEntry(const SearchEntry& entry)
{
    switch (entry.kind) {
    case SearchEntry::Kind::Command:
        // Through the action, so a checkable command toggles as from its menu;
        // a disabled action does not fire.
        registry_->refresh();
        if (QAction* action = registry_->action(entry.command)) {
            action->trigger();
        }
        break;
    case SearchEntry::Kind::Tool:
        if (ToolId tool; toolForName(entry.id, &tool)) {
            setActiveTool(tool);
        }
        break;
    case SearchEntry::Kind::Panel:
        if (QWidget* panel = showPanelForSearch(entry.id)) {
            flashWidget(panel);
        }
        break;
    case SearchEntry::Kind::Preference:
        showPreferences(entry.id);
        break;
    case SearchEntry::Kind::Control:
        if (QWidget* control = searchControlWidget(entry)) {
            focusControl(control);
            flashWidget(control);
        }
        break;
    }
}

void PicturaMainWindow::revealSearchEntry(const SearchEntry& entry)
{
    switch (entry.kind) {
    case SearchEntry::Kind::Command:
    case SearchEntry::Kind::Preference:
        registry_->refresh();
        revealMenuChain(menuBar(), registry_->action(entry.command));
        break;
    case SearchEntry::Kind::Tool:
        if (ToolId tool; toolbox_ && toolForName(entry.id, &tool)) {
            flashWidget(toolbox_->slotButton(toolInfo(tool).group));
        }
        break;
    case SearchEntry::Kind::Panel:
        flashWidget(showPanelForSearch(entry.id));
        break;
    case SearchEntry::Kind::Control:
        flashWidget(searchControlWidget(entry));
        break;
    }
}

void PicturaMainWindow::buildSearchHint(QStatusBar* bar)
{
    searchHint_ = new ToolHintBar(bar);
    searchHint_->setObjectName(QStringLiteral("statusSearchHint"));
    bar->addPermanentWidget(searchHint_);
    connect(registry_, &CommandRegistry::shortcutsChanged, this, [this](const QString& id) {
        if (id == QLatin1String(command_ids::HelpSearch)) {
            updateSearchHint();
        }
    });
    updateSearchHint();
}

void PicturaMainWindow::updateSearchHint()
{
    if (!searchHint_) {
        return;
    }
    const QList<QKeySequence> keys = registry_->shortcuts(QLatin1String(command_ids::HelpSearch));
    if (keys.isEmpty()) {
        searchHint_->setHints({}, QString());
        searchHint_->hide();
        return;
    }
    searchHint_->setHints({ToolHint{keys.first().toString(QKeySequence::NativeText), tr("Search")}},
                          QString());
    searchHint_->show();
}

} // namespace pictura
