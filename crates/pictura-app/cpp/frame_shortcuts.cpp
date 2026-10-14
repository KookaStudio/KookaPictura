#include "frame_includes.h"

#include "keyboard_shortcuts_dialog.h"

#include <QtGui/QShortcut>

namespace pictura {

void PicturaMainWindow::registerShortcutHandlers()
{
    registry_->setHandler(command_ids::EditKeyboardShortcuts, [this]() { showKeyboardShortcuts(); });
    registry_->setHandler(command_ids::WindowWorkspaceKeyboardShortcuts,
                          [this]() { showKeyboardShortcuts(); });
}

// Runs before the menus and the toolbox are built, so both pick the
// overrides up as they are created. An id or tool this build no longer has
// is dropped, losing only its override.
void PicturaMainWindow::applyShortcutOverrides(const SessionState& state)
{
    for (auto it = state.shortcuts.cbegin(); it != state.shortcuts.cend(); ++it) {
        QList<QKeySequence> bindings;
        for (const QString& binding : it.value()) {
            const QKeySequence sequence(binding, QKeySequence::PortableText);
            if (!sequence.isEmpty()) {
                bindings << sequence;
            }
        }
        registry_->setShortcuts(it.key(), bindings);
    }
    resetToolShortcuts();
    for (auto it = state.toolShortcuts.cbegin(); it != state.toolShortcuts.cend(); ++it) {
        for (ToolId id : allToolIds()) {
            if (toolIdName(id) == it.key()) {
                setToolShortcut(id, it.value().isEmpty() ? QChar() : it.value().at(0).toUpper());
                break;
            }
        }
    }
}

// One plain and one Shift shortcut per distinct slot letter; the toolbox
// resolves the letter to its group and honours `Use Shift Key For Tool
// Switch`. A key a menu command holds stays the command's: two shortcuts on
// one key are ambiguous and neither fires. (The shortcut editor prevents
// this; a store from an older build may not have.)
void PicturaMainWindow::rebuildToolShortcuts()
{
    qDeleteAll(toolKeyShortcuts_);
    toolKeyShortcuts_.clear();
    for (const QChar key : toolShortcutKeys()) {
        for (const bool shift : {false, true}) {
            const QKeySequence sequence = shift
                ? QKeySequence(Qt::SHIFT | Qt::Key(key.toUpper().unicode()))
                : QKeySequence(QString(key));
            if (!registry_->commandForShortcut(sequence).isEmpty()) {
                continue;
            }
            auto* shortcut = new QShortcut(sequence, this);
            connect(shortcut, &QShortcut::activated, this, [this, key, shift]() {
                if (toolbox_) {
                    toolbox_->handleToolKey(key, shift);
                }
            });
            toolKeyShortcuts_ << shortcut;
        }
    }
}

void PicturaMainWindow::showKeyboardShortcuts()
{
    KeyboardShortcutsDialog dialog(registry_, this);
    if (dialog.exec() != QDialog::Accepted) {
        return;
    }
    rebuildToolShortcuts();
    if (toolbox_) {
        toolbox_->refreshToolShortcuts();
    }
    saveSession();
}

} // namespace pictura
