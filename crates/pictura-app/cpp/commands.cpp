#include "commands.h"

#include "pictura_app/src/cxxqt_object/command_catalogue.cxxqt.h"

#include <QtCore/QDebug>
#include <QtCore/QEvent>
#include <QtGui/QAction>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>

#include <utility>

namespace pictura {

namespace {

QList<QKeySequence> bindingList(const QKeySequence& shortcut)
{
    return shortcut.isEmpty() ? QList<QKeySequence>() : QList<QKeySequence>{shortcut};
}

// A modal replay that falls back to the command's normal handler cannot report
// the entered values yet, but it can still tell an accepted dialog from a
// cancelled one.
class ModalDialogWatcher : public QObject {
public:
    bool sawDialog = false;
    bool accepted = false;

    bool eventFilter(QObject* watched, QEvent* event) override
    {
        if (event->type() == QEvent::Show) {
            if (auto* dialog = qobject_cast<QDialog*>(watched); dialog && dialog->isModal()) {
                sawDialog = true;
                connect(dialog, &QDialog::finished, this,
                        [this](int result) { accepted = result == QDialog::Accepted; });
            }
        }
        return QObject::eventFilter(watched, event);
    }
};

// Menu separators are layout, not commands, so the Rust catalogue does not
// carry them. `beforePath` is the command the separator precedes.
struct SeparatorRow {
    QStringList beforePath;
    QStringList path;
};

const SeparatorRow kSeparators[] = {
    {{QStringLiteral("File"), QStringLiteral("Close")}, {QStringLiteral("File")}},
    {{QStringLiteral("File"), QStringLiteral("Save")}, {QStringLiteral("File")}},
    {{QStringLiteral("File"), QStringLiteral("Place…")}, {QStringLiteral("File")}},
    {{QStringLiteral("File"), QStringLiteral("File Info…")}, {QStringLiteral("File")}},
    {{QStringLiteral("File"), QStringLiteral("Exit")}, {QStringLiteral("File")}},
    {{QStringLiteral("Edit"), QStringLiteral("Fade…")}, {QStringLiteral("Edit")}},
    {{QStringLiteral("Edit"), QStringLiteral("Cut")}, {QStringLiteral("Edit")}},
    {{QStringLiteral("Edit"), QStringLiteral("Check Spelling…")}, {QStringLiteral("Edit")}},
    {{QStringLiteral("Edit"), QStringLiteral("Fill…")}, {QStringLiteral("Edit")}},
    {{QStringLiteral("Edit"), QStringLiteral("Seam-Aware Scale")}, {QStringLiteral("Edit")}},
    {{QStringLiteral("Edit"), QStringLiteral("Transform"), QStringLiteral("Scale")}, {QStringLiteral("Edit"), QStringLiteral("Transform")}},
    {{QStringLiteral("Edit"), QStringLiteral("Transform"), QStringLiteral("Rotate 180°")}, {QStringLiteral("Edit"), QStringLiteral("Transform")}},
    {{QStringLiteral("Edit"), QStringLiteral("Transform"), QStringLiteral("Flip Horizontal")}, {QStringLiteral("Edit"), QStringLiteral("Transform")}},
    {{QStringLiteral("Edit"), QStringLiteral("Define Brush Preset…")}, {QStringLiteral("Edit")}},
    {{QStringLiteral("Edit"), QStringLiteral("Purge"), QStringLiteral("Undo")}, {QStringLiteral("Edit")}},
    {{QStringLiteral("Edit"), QStringLiteral("Adobe PDF Presets…")}, {QStringLiteral("Edit")}},
    {{QStringLiteral("Edit"), QStringLiteral("Color Settings…")}, {QStringLiteral("Edit")}},
    {{QStringLiteral("Edit"), QStringLiteral("Keyboard Shortcuts…")}, {QStringLiteral("Edit")}},
    {{QStringLiteral("Image"), QStringLiteral("Mode"), QStringLiteral("8 Bits/Channel")}, {QStringLiteral("Image"), QStringLiteral("Mode")}},
    {{QStringLiteral("Image"), QStringLiteral("Mode"), QStringLiteral("Color Table")}, {QStringLiteral("Image"), QStringLiteral("Mode")}},
    {{QStringLiteral("Image"), QStringLiteral("Adjustments"), QStringLiteral("Invert")}, {QStringLiteral("Image"), QStringLiteral("Adjustments")}},
    {{QStringLiteral("Image"), QStringLiteral("Adjustments"), QStringLiteral("Shadows/Highlights")}, {QStringLiteral("Image"), QStringLiteral("Adjustments")}},
    {{QStringLiteral("Image"), QStringLiteral("Adjustments"), QStringLiteral("Desaturate")}, {QStringLiteral("Image"), QStringLiteral("Adjustments")}},
    {{QStringLiteral("Image"), QStringLiteral("Auto Tone")}, {QStringLiteral("Image")}},
    {{QStringLiteral("Image"), QStringLiteral("Image Size…")}, {QStringLiteral("Image")}},
    {{QStringLiteral("Image"), QStringLiteral("Image Rotation"), QStringLiteral("180°")}, {QStringLiteral("Image")}},
    {{QStringLiteral("Image"), QStringLiteral("Crop")}, {QStringLiteral("Image")}},
    {{QStringLiteral("Image"), QStringLiteral("Variables"), QStringLiteral("Define…")}, {QStringLiteral("Image")}},
    {{QStringLiteral("Image"), QStringLiteral("Trap…")}, {QStringLiteral("Image")}},
    {{QStringLiteral("Layer"), QStringLiteral("Duplicate Layer…")}, {QStringLiteral("Layer")}},
    {{QStringLiteral("Layer"), QStringLiteral("Layer Style"), QStringLiteral("Blending Options…")}, {QStringLiteral("Layer")}},
    {{QStringLiteral("Layer"), QStringLiteral("Layer Style"), QStringLiteral("Copy Layer Style")}, {QStringLiteral("Layer"), QStringLiteral("Layer Style")}},
    {{QStringLiteral("Layer"), QStringLiteral("New Fill Layer"), QStringLiteral("Solid Color…")}, {QStringLiteral("Layer")}},
    {{QStringLiteral("Layer"), QStringLiteral("Layer Mask"), QStringLiteral("Reveal All")}, {QStringLiteral("Layer")}},
    {{QStringLiteral("Layer"), QStringLiteral("Smart Objects"), QStringLiteral("Convert to Smart Object")}, {QStringLiteral("Layer")}},
    {{QStringLiteral("Layer"), QStringLiteral("Copy Shape Attributes")}, {QStringLiteral("Layer")}},
    {{QStringLiteral("Layer"), QStringLiteral("Group Layers")}, {QStringLiteral("Layer")}},
    {{QStringLiteral("Layer"), QStringLiteral("Merge Layers")}, {QStringLiteral("Layer")}},
    {{QStringLiteral("Layer"), QStringLiteral("Matting"), QStringLiteral("Defringe…")}, {QStringLiteral("Layer")}},
    {{QStringLiteral("Select"), QStringLiteral("Reselect")}, {QStringLiteral("Select")}},
    {{QStringLiteral("Select"), QStringLiteral("All Layers")}, {QStringLiteral("Select")}},
    {{QStringLiteral("Select"), QStringLiteral("Color Range…")}, {QStringLiteral("Select")}},
    {{QStringLiteral("Select"), QStringLiteral("Modify"), QStringLiteral("Border…")}, {QStringLiteral("Select")}},
    {{QStringLiteral("Select"), QStringLiteral("Grow")}, {QStringLiteral("Select")}},
    {{QStringLiteral("Select"), QStringLiteral("Transform Selection")}, {QStringLiteral("Select")}},
    {{QStringLiteral("Filter"), QStringLiteral("Filter Gallery…")}, {QStringLiteral("Filter")}},
    {{QStringLiteral("Filter"), QStringLiteral("Artistic"), QStringLiteral("Colored Pencil")}, {QStringLiteral("Filter")}},
    {{QStringLiteral("Filter"), QStringLiteral("Extract…")}, {QStringLiteral("Filter")}},
    {{QStringLiteral("Filter"), QStringLiteral("Browse Filters Online…")}, {QStringLiteral("Filter")}},
    {{QStringLiteral("View"), QStringLiteral("Zoom In")}, {QStringLiteral("View")}},
    {{QStringLiteral("View"), QStringLiteral("Screen Mode"), QStringLiteral("Standard Screen Mode")}, {QStringLiteral("View")}},
    {{QStringLiteral("View"), QStringLiteral("Rulers")}, {QStringLiteral("View")}},
    {{QStringLiteral("View"), QStringLiteral("Lock Slices")}, {QStringLiteral("View")}},
    {{QStringLiteral("View"), QStringLiteral("Show"), QStringLiteral("Selection Edges")}, {QStringLiteral("View")}},
    {{QStringLiteral("View"), QStringLiteral("Align")}, {QStringLiteral("View")}},
    {{QStringLiteral("View"), QStringLiteral("Tool Hints")}, {QStringLiteral("View")}},
    {{QStringLiteral("Window"), QStringLiteral("Workspace"), QStringLiteral("New Workspace…")}, {QStringLiteral("Window"), QStringLiteral("Workspace")}},
    {{QStringLiteral("Window"), QStringLiteral("Workspace"), QStringLiteral("Keyboard Shortcuts & Menus…")}, {QStringLiteral("Window"), QStringLiteral("Workspace")}},
    {{QStringLiteral("Window"), QStringLiteral("Panels"), QStringLiteral("3D")}, {QStringLiteral("Window")}},
    {{QStringLiteral("Window"), QStringLiteral("Options")}, {QStringLiteral("Window")}},
    {{QStringLiteral("Help"), QStringLiteral("Kooka Pictura Help")}, {QStringLiteral("Help")}},
    {{QStringLiteral("Help"), QStringLiteral("About Plug-in")}, {QStringLiteral("Help")}},
    {{QStringLiteral("Help"), QStringLiteral("About Kooka Pictura")}, {QStringLiteral("Help")}},
    {{QStringLiteral("Help"), QStringLiteral("Developer"), QStringLiteral("Design System...")}, {QStringLiteral("Help")}},
};
constexpr int kSeparatorCount = int(sizeof(kSeparators) / sizeof(kSeparators[0]));

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

void CommandRegistry::setContextHandler(const QString& id,
                                        std::function<void(const CommandContext&)> handler)
{
    contextHandlers_.insert(id, std::move(handler));
}

void CommandRegistry::setExecutor(const QString& id, CommandExecutor executor)
{
    executors_.insert(id, std::move(executor));
}

void CommandRegistry::setRecordingSink(CommandRecordingSink sink)
{
    recordingSink_ = std::move(sink);
}

void CommandRegistry::recordCommand(const QString& id, const CommandParams& params)
{
    if (recordingSink_) {
        recordingSink_(id, params);
    }
}

bool CommandRegistry::executeRecorded(const QString& id, const CommandParams& params,
                                      CommandMode mode)
{
    return executeRecordedImpl(id, params, mode, nullptr);
}

bool CommandRegistry::executeRecorded(const QString& id, CommandParams* params,
                                      CommandMode mode)
{
    if (!params) {
        lastError_ = QStringLiteral("no parameters for command %1").arg(id);
        return false;
    }
    return executeRecordedImpl(id, *params, mode, params);
}

bool CommandRegistry::executeRecordedImpl(const QString& id, const CommandParams& params,
                                          CommandMode mode, CommandParams* editedOut)
{
    if (mode == CommandMode::Modal) {
        auto context = contextHandlers_.find(id);
        if (context != contextHandlers_.end()) {
            CommandParams edited = params;
            bool applied = false;
            context.value()(CommandContext{mode, params, &edited, &applied});
            if (!applied) {
                lastError_ = QStringLiteral("command %1 was cancelled").arg(id);
                qWarning().noquote() << lastError_;
                return false;
            }
            if (editedOut) {
                *editedOut = edited;
            }
            lastError_.clear();
            return true;
        }
        // A handler opens the command's normal dialog. It cannot report a
        // cancel or the entered values yet; commands that need that register a
        // context handler above.
        auto handler = handlers_.find(id);
        if (handler != handlers_.end()) {
            ModalDialogWatcher watcher;
            if (qApp) {
                qApp->installEventFilter(&watcher);
            }
            handler.value()();
            if (qApp) {
                qApp->removeEventFilter(&watcher);
            }
            if (watcher.sawDialog && !watcher.accepted) {
                lastError_ = QStringLiteral("command %1 was cancelled").arg(id);
                qWarning().noquote() << lastError_;
                return false;
            }
            lastError_.clear();
            return true;
        }
        auto executor = executors_.find(id);
        if (executor != executors_.end()) {
            if (!executor.value()(params)) {
                lastError_ = QStringLiteral("command %1 refused replay").arg(id);
                qWarning().noquote() << lastError_;
                return false;
            }
            lastError_.clear();
            return true;
        }
        lastError_ = QStringLiteral("no modal handler for command %1").arg(id);
        qWarning().noquote() << lastError_;
        return false;
    }
    auto executor = executors_.find(id);
    if (executor == executors_.end()) {
        lastError_ = QStringLiteral("no executor for command %1").arg(id);
        qWarning().noquote() << lastError_;
        return false;
    }
    if (!executor.value()(params)) {
        lastError_ = QStringLiteral("command %1 refused replay").arg(id);
        qWarning().noquote() << lastError_;
        return false;
    }
    lastError_.clear();
    return true;
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
    return handlers_.contains(id) || contextHandlers_.contains(id);
}

void CommandRegistry::setEnabledProvider(const QString& id, std::function<bool()> enabled)
{
    enabledProviders_.insert(id, std::move(enabled));
}

void CommandRegistry::setBlockedProvider(std::function<bool(const CommandSpec&)> blocked)
{
    blockedProvider_ = std::move(blocked);
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
            Q_EMIT shortcutsChanged(id);
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
        if (enabled && blockedProvider_ && blockedProvider_(spec)) {
            enabled = false;
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
    if (blockedProvider_) {
        for (const Entry& entry : entries_) {
            if (entry.spec.id == id && blockedProvider_(entry.spec)) {
                return false;
            }
        }
    }
    auto context = contextHandlers_.find(id);
    if (context != contextHandlers_.end()) {
        context.value()(CommandContext{CommandMode::Ui, CommandParams{}});
        return true;
    }
    auto handler = handlers_.find(id);
    if (handler == handlers_.end()) {
        return false;
    }
    handler.value()();
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

void addDefaultCommands(CommandRegistry& registry)
{
    for (const CommandSpecRow& row : command_catalogue()) {
        for (int i = 0; i < kSeparatorCount; ++i) {
            if (kSeparators[i].beforePath == row.path) {
                registry.addSeparator(kSeparators[i].path);
            }
        }
        if (row.id.isEmpty()) {
            continue;
        }
        CommandSpec spec;
        spec.id = row.id;
        spec.path = row.path;
        spec.label = row.label;
        spec.shortcut =
            row.shortcut.isEmpty() ? QKeySequence() : QKeySequence(row.shortcut);
        spec.implemented = row.implemented;
        spec.checkable = row.checkable;
        registry.add(spec);
    }
}

QString commandIdForPath(const QStringList& path)
{
    return command_id_for_path(path);
}

} // namespace pictura

#include "moc_commands.cpp"
