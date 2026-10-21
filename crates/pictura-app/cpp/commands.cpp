#include "commands.h"

#include <QtGui/QAction>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>

#include <utility>

namespace pictura {

namespace {

QList<QKeySequence> bindingList(const QKeySequence& shortcut)
{
    return shortcut.isEmpty() ? QList<QKeySequence>() : QList<QKeySequence>{shortcut};
}

} // namespace

CommandRegistry::CommandRegistry(QObject* parent) : QObject(parent) {}

void CommandRegistry::add(const CommandSpec& spec)
{
    entries_.push_back(Entry{spec, nullptr, bindingList(spec.shortcut), bindingList(spec.shortcut)});
}

void CommandRegistry::add(const QString& id, const QStringList& path, const QString& label,
                          const QKeySequence& shortcut, bool implemented)
{
    CommandSpec spec;
    spec.id = id;
    spec.path = path;
    spec.label = label;
    spec.shortcut = shortcut;
    spec.implemented = implemented;
    entries_.push_back(Entry{spec, nullptr, bindingList(shortcut), bindingList(shortcut)});
}

void CommandRegistry::addSeparator(const QStringList& path)
{
    CommandSpec spec;
    spec.path = path;
    entries_.push_back(Entry{spec, nullptr, {}, {}});
}

void CommandRegistry::setHandler(const QString& id, std::function<void()> handler)
{
    handlers_.insert(id, std::move(handler));
}

void CommandRegistry::setImplemented(const QString& id, bool implemented)
{
    for (Entry& entry : entries_) {
        if (!entry.spec.id.isEmpty() && entry.spec.id == id) {
            entry.spec.implemented = implemented;
            return;
        }
    }
}

bool CommandRegistry::hasHandler(const QString& id) const
{
    return handlers_.contains(id);
}

void CommandRegistry::setEnabledProvider(const QString& id, std::function<bool()> enabled)
{
    enabledProviders_.insert(id, std::move(enabled));
}

void CommandRegistry::setCheckedProvider(const QString& id, std::function<bool()> checked)
{
    checkedProviders_.insert(id, std::move(checked));
}

void CommandRegistry::setLabelProvider(const QString& id, std::function<QString()> label)
{
    labelProviders_.insert(id, std::move(label));
}

const CommandRegistry::Entry* CommandRegistry::entry(const QString& id) const
{
    for (const Entry& entry : entries_) {
        if (!entry.spec.id.isEmpty() && entry.spec.id == id) {
            return &entry;
        }
    }
    return nullptr;
}

QAction* CommandRegistry::action(const QString& id) const
{
    const Entry* found = entry(id);
    return found ? found->action : nullptr;
}

QList<QKeySequence> CommandRegistry::shortcuts(const QString& id) const
{
    const Entry* found = entry(id);
    return found ? found->shortcuts : QList<QKeySequence>();
}

QList<QKeySequence> CommandRegistry::defaultShortcuts(const QString& id) const
{
    const Entry* found = entry(id);
    return found ? found->defaults : QList<QKeySequence>();
}

void CommandRegistry::setShortcuts(const QString& id, const QList<QKeySequence>& shortcuts)
{
    for (Entry& entry : entries_) {
        if (!entry.spec.id.isEmpty() && entry.spec.id == id) {
            entry.shortcuts = shortcuts;
            if (entry.action) {
                entry.action->setShortcuts(shortcuts);
            }
            return;
        }
    }
}

QString CommandRegistry::commandForShortcut(const QKeySequence& shortcut) const
{
    if (shortcut.isEmpty()) {
        return QString();
    }
    for (const Entry& entry : entries_) {
        if (!entry.spec.id.isEmpty() && entry.shortcuts.contains(shortcut)) {
            return entry.spec.id;
        }
    }
    return QString();
}

QHash<QString, QList<QKeySequence>> CommandRegistry::shortcutOverrides() const
{
    QHash<QString, QList<QKeySequence>> overrides;
    for (const Entry& entry : entries_) {
        if (!entry.spec.id.isEmpty() && entry.shortcuts != entry.defaults) {
            overrides.insert(entry.spec.id, entry.shortcuts);
        }
    }
    return overrides;
}

void CommandRegistry::buildMenuBar(QMenuBar* menuBar)
{
    menuBar_ = menuBar;
    if (!menuBar_) {
        return;
    }

    menuBar->clear();
    const QList<QMenu*> previous = menuBar->findChildren<QMenu*>(Qt::FindDirectChildrenOnly);
    for (QMenu* menu : previous) {
        delete menu;
    }

    QHash<QString, QMenu*> menus;
    for (Entry& entry : entries_) {
        entry.action = nullptr;
        const QStringList& path = entry.spec.path;
        if (path.isEmpty()) {
            continue;
        }

        const bool separator = entry.spec.id.isEmpty();
        const int menuCount = separator ? path.size() : path.size() - 1;

        QMenu* parent = nullptr;
        QString key;
        for (int i = 0; i < menuCount; ++i) {
            key = (i == 0) ? path.at(0) : key + QLatin1Char('/') + path.at(i);
            QMenu* menu = menus.value(key);
            if (!menu) {
                menu = parent ? parent->addMenu(path.at(i)) : menuBar->addMenu(path.at(i));
                menus.insert(key, menu);
                connect(menu, &QMenu::aboutToShow, this, &CommandRegistry::refresh);
            }
            parent = menu;
        }

        if (!parent) {
            continue;
        }
        if (separator) {
            parent->addSeparator();
            continue;
        }

        QAction* item = parent->addAction(path.last());
        item->setData(entry.spec.id);
        item->setShortcuts(entry.shortcuts);
        item->setCheckable(entry.spec.checkable);
        const QString id = entry.spec.id;
        connect(item, &QAction::triggered, this, [this, id] { dispatch(id); });
        entry.action = item;
    }

    refresh();
}

void CommandRegistry::refresh()
{
    ++refreshCount_;
    for (Entry& entry : entries_) {
        if (!entry.action) {
            continue;
        }
        const CommandSpec& spec = entry.spec;

        bool enabled = spec.implemented && hasHandler(spec.id);
        if (enabledProviders_.contains(spec.id)) {
            enabled = enabled && enabledProviders_[spec.id]();
        }
        entry.action->setEnabled(enabled);

        if (spec.checkable) {
            const bool checked = checkedProviders_.contains(spec.id) && checkedProviders_[spec.id]();
            entry.action->setChecked(checked);
        }

        const QString text =
            labelProviders_.contains(spec.id) ? labelProviders_[spec.id]() : spec.label;
        entry.action->setText(text);
    }
}

bool CommandRegistry::dispatch(const QString& id)
{
    auto it = handlers_.find(id);
    if (it == handlers_.end()) {
        return false;
    }
    it.value()();
    return true;
}

QStringList CommandRegistry::topLevelTitles() const
{
    QStringList titles;
    for (const Entry& entry : entries_) {
        if (entry.spec.path.isEmpty()) {
            continue;
        }
        const QString& top = entry.spec.path.first();
        if (!titles.contains(top)) {
            titles.append(top);
        }
    }
    return titles;
}

QList<CommandInfo> CommandRegistry::describe() const
{
    QList<CommandInfo> infos;
    for (const Entry& entry : entries_) {
        if (entry.spec.id.isEmpty()) {
            continue;
        }
        CommandInfo info;
        info.id = entry.spec.id;
        info.path = entry.spec.path;
        info.label = entry.spec.label;
        info.implemented = entry.spec.implemented;
        if (entry.action) {
            info.enabled = entry.action->isEnabled();
            info.checked = entry.action->isChecked();
        } else {
            bool enabled = entry.spec.implemented && hasHandler(entry.spec.id);
            if (enabledProviders_.contains(entry.spec.id)) {
                enabled = enabled && enabledProviders_.value(entry.spec.id)();
            }
            info.enabled = enabled;
            info.checked = entry.spec.checkable && checkedProviders_.contains(entry.spec.id)
                           && checkedProviders_.value(entry.spec.id)();
        }
        infos.append(info);
    }
    return infos;
}

QString commandIdForPath(const QStringList& path)
{
    QString id;
    for (const QString& segment : path) {
        for (const QChar c : segment) {
            if (c.isLetterOrNumber()) {
                id += c.toLower();
            } else if (c == QLatin1Char(' ') || c == QLatin1Char('-')) {
                id += QLatin1Char('.');
            }
        }
        id += QLatin1Char('.');
    }
    id.chop(1);
    return id;
}

} // namespace pictura

#include "moc_commands.cpp"
