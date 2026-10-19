#include <QtCore/QRect>
#include "frame_includes.h"

#include "workspace_store.h"

#include <QtCore/QFile>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QLineEdit>

namespace pictura {
namespace {

struct BuiltinDef {
    const char* id;
    const char* name;
};

const BuiltinDef kBuiltins[] = {
    {"essentials", "Essentials"},
    {"painting", "Painting"},
    {"photography", "Photography"},
    {"typography", "Typography"},
};

// docs: workspace names are user content; cap the length so a paste cannot
// create an unusable entry.
constexpr int kWorkspaceNameCap = 64;
const QString kDefaultWorkspace = QStringLiteral("Essentials");
const QString kBuiltinKind = QStringLiteral("builtin");
const QString kUserKind = QStringLiteral("user");

QJsonArray rectJson(const QRect& rect)
{
    QJsonArray array;
    array.append(rect.x());
    array.append(rect.y());
    array.append(rect.width());
    array.append(rect.height());
    return array;
}

} // namespace

QJsonArray PicturaMainWindow::serializeWorkspaceLayout() const
{
    // v10: the ordered per-column layout, in central-splitter order, each with
    // its normal-mode width, rail mode, and floating state/geometry. The tools
    // column is recorded with a `tools` marker and handled before the detached
    // branch so a floating Tools column is never dropped. Floating groups keep
    // their own entries inside their owner column's `groups` array.
    QJsonArray columns;
    int order = 0;
    for (PanelColumn* column : allPanelColumns()) {
        if (!column) {
            continue;
        }
        // A detached (cancelled-float) column is still reachable through its
        // overlay until the event loop runs; it is not a live pane.
        if (centerSplitter_->indexOf(column) < 0 && !column->isColumnFloating()) {
            continue;
        }
        QJsonObject entry;
        entry.insert(QStringLiteral("side"),
                     column->side() == PanelSide::Left ? QStringLiteral("left")
                                                       : QStringLiteral("right"));
        entry.insert(QStringLiteral("order"), order++);
        entry.insert(QStringLiteral("width"), column->persistedWidth());
        entry.insert(QStringLiteral("railMode"),
                     column->railMode() ? QStringLiteral("iconic")
                                        : QStringLiteral("normal"));
        if (column->isColumnFloating()) {
            entry.insert(QStringLiteral("floating"), true);
            entry.insert(QStringLiteral("floatGeometry"),
                         rectJson(column->columnFloatGeometry()));
        }
        if (column->isToolsColumn()) {
            entry.insert(QStringLiteral("tools"), true);
            entry.insert(QStringLiteral("groups"), QJsonArray());
        } else {
            entry.insert(QStringLiteral("groups"), column->savePanelState());
        }
        columns.append(entry);
    }
    return columns;
}

int PicturaMainWindow::workspaceIndexOf(const QString& name) const
{
    for (int i = 0; i < workspaceStore_.workspaces.size(); ++i) {
        if (workspaceStore_.workspaces.at(i).name == name) {
            return i;
        }
    }
    return -1;
}

void PicturaMainWindow::initWorkspaces(bool sessionHadLayout)
{
    WorkspaceStore loaded;
    const bool haveStore = loadWorkspaceStore(&loaded);
    workspaceStore_ = haveStore ? loaded : WorkspaceStore{};

    if (!haveStore || workspaceStore_.workspaces.isEmpty()) {
        // First run (or a missing/unusable store): seed the built-in presets.
        workspaceStore_.workspaces.clear();
        for (const BuiltinDef& def : kBuiltins) {
            WorkspaceRecord record;
            record.id = QString::fromLatin1(def.id);
            record.name = QString::fromLatin1(def.name);
            record.kind = kBuiltinKind;
            record.factory = workspacePresets(record.name);
            workspaceStore_.workspaces.append(record);
        }
    } else {
        // The loaded record set is authoritative; a preset deleted last session
        // stays gone. Builtins still take their factory from code so Reset works.
        for (WorkspaceRecord& record : workspaceStore_.workspaces) {
            if (record.kind == kBuiltinKind) {
                record.factory = workspacePresets(record.name);
            }
        }
    }

    if (workspaceIndexOf(workspaceStore_.activeWorkspace) < 0) {
        workspaceStore_.activeWorkspace = workspaceStore_.workspaces.isEmpty()
                                              ? kDefaultWorkspace
                                              : workspaceStore_.workspaces.first().name;
    }

    const int active = workspaceIndexOf(workspaceStore_.activeWorkspace);
    if (active >= 0) {
        WorkspaceRecord& record = workspaceStore_.workspaces[active];
        if (!haveStore || sessionHadLayout) {
            // D3: the restored live layout is the active workspace's arrangement.
            record.layout = serializeWorkspaceLayout();
            record.hasLayout = true;
        } else {
            // The session lost its layout: keep the active workspace's own saved
            // arrangement instead of overwriting it with the fresh default.
            const QJsonArray applied = record.hasLayout
                                           ? record.layout
                                           : (record.kind == kBuiltinKind
                                                  ? workspacePresets(record.name)
                                                  : record.factory);
            regroupAndApply(applied);
            record.layout = serializeWorkspaceLayout();
            record.hasLayout = true;
        }
    }

    workspacesInitialized_ = true;
    saveWorkspaceStore(workspaceStore_);
    registry_->refresh();
    attachWorkspaceSwitcher();
}

void PicturaMainWindow::attachWorkspaceSwitcher()
{
    if (!optionsBar_) {
        return;
    }
    optionsBar_->setWorkspaceMenu(workspaceMenu());
    updateWorkspaceSwitcher();
    if (QMenu* menu = workspaceMenu()) {
        refreshWorkspaceMenu(menu);
    }
}

void PicturaMainWindow::updateWorkspaceSwitcher()
{
    if (optionsBar_) {
        optionsBar_->setActiveWorkspace(workspaceStore_.activeWorkspace);
    }
}

void PicturaMainWindow::snapshotActiveWorkspace()
{
    if (!workspacesInitialized_) {
        return;
    }
    const int index = workspaceIndexOf(workspaceStore_.activeWorkspace);
    if (index < 0) {
        return;
    }
    WorkspaceRecord& record = workspaceStore_.workspaces[index];
    const QJsonArray layout = serializeWorkspaceLayout();
    if (record.hasLayout && record.layout == layout) {
        return;
    }
    record.layout = layout;
    record.hasLayout = true;
    saveWorkspaceStore(workspaceStore_);
}

void PicturaMainWindow::switchWorkspace(const QString& name)
{
    if (name.isEmpty()) {
        return;
    }
    if (name == workspaceStore_.activeWorkspace) {
        // A re-select restores a toggle that was switched off without a switch.
        registry_->refresh();
        return;
    }
    const int target = workspaceIndexOf(name);
    if (target < 0) {
        return;
    }

    // Remember the outgoing workspace's live arrangement (D3).
    const int outgoing = workspaceIndexOf(workspaceStore_.activeWorkspace);
    if (outgoing >= 0) {
        workspaceStore_.workspaces[outgoing].layout = serializeWorkspaceLayout();
        workspaceStore_.workspaces[outgoing].hasLayout = true;
    }

    workspaceStore_.activeWorkspace = name;
    const WorkspaceRecord& record = workspaceStore_.workspaces.at(target);
    if (record.hasLayout && !record.layout.isEmpty()) {
        regroupAndApply(record.layout);
    } else if (record.kind == kBuiltinKind) {
        regroupAndApply(workspacePresets(name));
    } else {
        regroupAndApply(record.factory);
    }
    saveWorkspaceStore(workspaceStore_);
    registry_->refresh();
    updateWorkspaceSwitcher();
}

void PicturaMainWindow::resetWorkspace()
{
    const int index = workspaceIndexOf(workspaceStore_.activeWorkspace);
    if (index < 0) {
        return;
    }
    WorkspaceRecord& record = workspaceStore_.workspaces[index];
    const QJsonArray factory = record.kind == kBuiltinKind ? workspacePresets(record.name)
                                                           : record.factory;
    if (factory.isEmpty()) {
        return;
    }
    record.layout = factory;
    record.hasLayout = true;
    regroupAndApply(factory);
    saveWorkspaceStore(workspaceStore_);
    registry_->refresh();
    updateWorkspaceSwitcher();
}

QString PicturaMainWindow::workspaceNameError(const QString& name) const
{
    const QString trimmed = name.trimmed();
    if (trimmed.isEmpty()) {
        return tr("Enter a workspace name.");
    }
    if (trimmed.size() > kWorkspaceNameCap) {
        return tr("Workspace names cannot exceed %1 characters.").arg(kWorkspaceNameCap);
    }
    const QString duplicate = tr("A workspace named \"%1\" already exists.").arg(trimmed);
    if (workspaceIndexOf(trimmed) >= 0) {
        return duplicate;
    }
    // The four preset names stay reserved even when their record was deleted, so
    // a user workspace can never shadow a preset that may be restored.
    for (const BuiltinDef& def : kBuiltins) {
        if (trimmed == QString::fromLatin1(def.name)) {
            return duplicate;
        }
    }
    return QString();
}

bool PicturaMainWindow::createWorkspace(const QString& name)
{
    if (!workspaceNameError(name).isEmpty()) {
        return false;
    }
    const QString trimmed = name.trimmed();

    QString id;
    for (int n = 1; id.isEmpty(); ++n) {
        const QString candidate = QStringLiteral("user-") + QString::number(n);
        bool used = false;
        for (const WorkspaceRecord& record : workspaceStore_.workspaces) {
            if (record.id == candidate) {
                used = true;
                break;
            }
        }
        if (!used) {
            id = candidate;
        }
    }

    const QJsonArray layout = serializeWorkspaceLayout();
    WorkspaceRecord record;
    record.id = id;
    record.name = trimmed;
    record.kind = kUserKind;
    record.layout = layout;
    record.factory = layout;
    record.hasLayout = true;
    workspaceStore_.workspaces.append(record);
    workspaceStore_.activeWorkspace = trimmed;

    saveWorkspaceStore(workspaceStore_);
    registry_->refresh();
    updateWorkspaceSwitcher();
    return true;
}

bool PicturaMainWindow::deleteWorkspace(const QString& name)
{
    // The active workspace is load-bearing (it owns the live layout).
    if (name.isEmpty() || name == workspaceStore_.activeWorkspace) {
        return false;
    }
    const int index = workspaceIndexOf(name);
    if (index < 0) {
        return false;
    }
    const QString id = workspaceStore_.workspaces.at(index).id;
    workspaceStore_.workspaces.removeAt(index);
    // The index rewrite drops the record from every listing; the per-id file is
    // removed too so it cannot resurface.
    QFile::remove(workspaceStoreDir() + QLatin1Char('/') + id + QStringLiteral(".json"));
    saveWorkspaceStore(workspaceStore_);
    registry_->refresh();
    updateWorkspaceSwitcher();
    return true;
}

QStringList PicturaMainWindow::userWorkspacesForTest() const
{
    QStringList names;
    for (const WorkspaceRecord& record : workspaceStore_.workspaces) {
        if (record.kind == kUserKind) {
            names.append(record.name);
        }
    }
    return names;
}

QMenu* PicturaMainWindow::workspaceMenu() const
{
    for (QMenu* menu : menuBar()->findChildren<QMenu*>()) {
        if (menu->title() == QStringLiteral("Workspace")) {
            return menu;
        }
    }
    return nullptr;
}

void PicturaMainWindow::refreshWorkspaceMenu(QMenu* menu)
{
    for (QAction* action : workspaceMenuActions_) {
        menu->removeAction(action);
        delete action;
    }
    workspaceMenuActions_.clear();

    // A preset the user deleted is hidden, not shown as a dead entry.
    static const struct {
        const char* command;
        const char* name;
    } presets[] = {
        {command_ids::WindowWorkspaceEssentials, "Essentials"},
        {command_ids::WindowWorkspacePainting, "Painting"},
        {command_ids::WindowWorkspacePhotography, "Photography"},
        {command_ids::WindowWorkspaceTypography, "Typography"},
    };
    for (const auto& preset : presets) {
        if (QAction* action = registry_->action(QString::fromLatin1(preset.command))) {
            // A preset shows only while its own builtin record exists; a user
            // workspace that happens to share the name must not reveal it.
            const int index = workspaceIndexOf(QString::fromLatin1(preset.name));
            action->setVisible(index >= 0
                               && workspaceStore_.workspaces.at(index).kind == kBuiltinKind);
        }
    }

    // User workspaces join the presets section: insert before the first separator
    // (the one after the preset commands), not into the actions section.
    QAction* before = nullptr;
    for (QAction* action : menu->actions()) {
        if (action->isSeparator()) {
            before = action;
            break;
        }
    }
    if (!before) {
        before = registry_->action(command_ids::WindowWorkspaceNew);
    }
    for (const WorkspaceRecord& record : workspaceStore_.workspaces) {
        if (record.kind != kUserKind) {
            continue;
        }
        auto* action = new QAction(record.name, menu);
        action->setCheckable(true);
        action->setChecked(workspaceStore_.activeWorkspace == record.name);
        const QString name = record.name;
        connect(action, &QAction::triggered, this, [this, name]() { switchWorkspace(name); });
        menu->insertAction(before, action);
        workspaceMenuActions_.append(action);
    }

    updateWorkspaceSwitcher();
}

int PicturaMainWindow::workspaceMenuSeparatorCountForTest() const
{
    int count = 0;
    if (QMenu* menu = workspaceMenu()) {
        for (QAction* action : menu->actions()) {
            if (action->isSeparator()) {
                ++count;
            }
        }
    }
    return count;
}

void PicturaMainWindow::refreshWorkspaceMenuForTest()
{
    if (QMenu* menu = workspaceMenu()) {
        refreshWorkspaceMenu(menu);
    }
}

QStringList PicturaMainWindow::workspaceMenuLabelsForTest() const
{
    QStringList labels;
    if (QMenu* menu = workspaceMenu()) {
        for (QAction* action : menu->actions()) {
            if (!action->isSeparator()) {
                labels.append(action->text());
            }
        }
    }
    return labels;
}

QStringList PicturaMainWindow::deleteWorkspaceChoices() const
{
    // Exactly the list the Delete chooser offers: every workspace but the active.
    QStringList names;
    for (const WorkspaceRecord& record : workspaceStore_.workspaces) {
        if (record.name != workspaceStore_.activeWorkspace) {
            names.append(record.name);
        }
    }
    return names;
}

void PicturaMainWindow::wireWorkspaceCommands()
{
    const struct {
        const char* command;
        const char* name;
    } presets[] = {
        {command_ids::WindowWorkspaceEssentials, "Essentials"},
        {command_ids::WindowWorkspacePainting, "Painting"},
        {command_ids::WindowWorkspacePhotography, "Photography"},
        {command_ids::WindowWorkspaceTypography, "Typography"},
    };
    for (const auto& preset : presets) {
        const QString command = QString::fromLatin1(preset.command);
        const QString name = QString::fromLatin1(preset.name);
        registry_->setHandler(command, [this, name]() { switchWorkspace(name); });
        registry_->setCheckedProvider(command, [this, name]() {
            return workspaceStore_.activeWorkspace == name;
        });
    }

    registry_->setHandler(command_ids::WindowWorkspaceReset,
                          [this]() { resetWorkspace(); });
    registry_->setLabelProvider(command_ids::WindowWorkspaceReset, [this]() {
        return tr("Reset ") + workspaceStore_.activeWorkspace;
    });

    registry_->setHandler(command_ids::WindowWorkspaceNew, [this]() {
        bool ok = false;
        const QString name =
            QInputDialog::getText(this, tr("New Workspace"), tr("Workspace name:"),
                                  QLineEdit::Normal, QString(), &ok);
        if (!ok) {
            return;
        }
        const QString error = workspaceNameError(name);
        if (!error.isEmpty()) {
            QMessageBox::warning(this, tr("New Workspace"), error);
            return;
        }
        createWorkspace(name);
    });

    registry_->setHandler(command_ids::WindowWorkspaceDelete, [this]() {
        const QStringList names = deleteWorkspaceChoices();
        if (names.isEmpty()) {
            return;
        }
        bool ok = false;
        const QString name =
            QInputDialog::getItem(this, tr("Delete Workspace"), tr("Workspace:"), names, 0,
                                  false, &ok);
        if (!ok) {
            return;
        }
        if (QMessageBox::question(this, tr("Delete Workspace"),
                                  tr("Delete workspace \"%1\"?").arg(name))
            == QMessageBox::Yes) {
            deleteWorkspace(name);
        }
    });

    if (QAction* newAction = registry_->action(command_ids::WindowWorkspaceNew)) {
        if (QMenu* menu = qobject_cast<QMenu*>(newAction->parent())) {
            connect(menu, &QMenu::aboutToShow, this,
                    [this, menu]() { refreshWorkspaceMenu(menu); });
        }
    }
}

} // namespace pictura
