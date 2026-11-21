// Window > Workspace: the built-in presets, Reset, and the user workspace
// commands. The workspaces themselves live in PanelHost.

#include <QtCore/QCoreApplication>
#include "menu_registrar.h"

#include "panel_host.h"

#include "commands.h"
#include <QtGui/QAction>

#include <QtWidgets/QInputDialog>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMessageBox>

namespace pictura {

void MenuRegistrar::wireWorkspaceCommands()
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
        commands_->setHandler(command, [this, name]() {
            if (panels_) {
                panels_->switchWorkspace(name);
            }
        });
        commands_->setCheckedProvider(command, [this, name]() {
            return panels_ && panels_->activeWorkspace() == name;
        });
    }

    commands_->setHandler(command_ids::WindowWorkspaceReset, [this]() {
        if (panels_) {
            panels_->resetWorkspace();
        }
    });
    commands_->setLabelProvider(command_ids::WindowWorkspaceReset, [this]() {
        return QCoreApplication::translate("PicturaCommands", "Reset %1")
            .arg(panels_ ? panels_->activeWorkspace() : QString());
    });

    commands_->setHandler(command_ids::WindowWorkspaceNew, [this]() {
        bool ok = false;
        const QString name =
            QInputDialog::getText(dialogParent(), tr("New Workspace"), tr("Workspace name:"),
                                  QLineEdit::Normal, QString(), &ok);
        if (!ok || !panels_) {
            return;
        }
        const QString error = panels_->workspaceNameError(name);
        if (!error.isEmpty()) {
            QMessageBox::warning(dialogParent(), tr("New Workspace"), error);
            return;
        }
        panels_->createWorkspace(name);
    });

    commands_->setHandler(command_ids::WindowWorkspaceDelete, [this]() {
        if (!panels_) {
            return;
        }
        const QStringList names = panels_->deleteWorkspaceChoices();
        if (names.isEmpty()) {
            return;
        }
        bool ok = false;
        const QString name =
            QInputDialog::getItem(dialogParent(), tr("Delete Workspace"), tr("Workspace:"), names,
                                  0, false, &ok);
        if (!ok) {
            return;
        }
        if (QMessageBox::question(dialogParent(), tr("Delete Workspace"),
                                  tr("Delete workspace \"%1\"?").arg(name))
            == QMessageBox::Yes) {
            panels_->deleteWorkspace(name);
        }
    });

    if (QAction* newAction = commands_->action(command_ids::WindowWorkspaceNew)) {
        if (QMenu* menu = qobject_cast<QMenu*>(newAction->parent())) {
            connect(menu, &QMenu::aboutToShow, this, [this, menu]() {
                if (panels_) {
                    panels_->refreshWorkspaceMenu(menu);
                }
            });
        }
    }
}

} // namespace pictura
