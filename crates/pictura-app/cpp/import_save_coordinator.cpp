// The file seam: open/save/import tokens, recent files, save-format options,
// smart-object edit sessions, and crash recovery live here. The frame keeps
// the chrome and forwards its entry points to this coordinator.

#include "import_save_coordinator.h"

#include "dialogs.h"
#include "document_registry.h"
#include "session.h"

#include <QtCore/QFileInfo>

namespace pictura {

ImportSaveCoordinator::ImportSaveCoordinator(DocumentRegistry& documents,
                                             QWidget* dialogParent, Hooks hooks,
                                             QObject* parent)
    : QObject(parent)
    , documents_(documents)
    , dialogParent_(dialogParent)
    , hooks_(std::move(hooks))
{
    connect(&documents_, &DocumentRegistry::aboutToRemoveDocument, this,
            [this](int index) { onAboutToRemoveDocument(index); });
}

ImportSaveCoordinator::~ImportSaveCoordinator()
{
    shutdownRecovery();
    for (const SmartObjectEditSession& session : editSessions_) {
        delete session.temp;
    }
    editSessions_.clear();
}

void ImportSaveCoordinator::saveState(SessionState& state) const
{
    state.recent = recent_;
    state.saveFormatOptions = saveFormatOptions_;
    state.autoSaveRecovery = autoSaveRecovery_;
    state.autoSaveMinutes = autoSaveMinutes_;
}

void ImportSaveCoordinator::loadState(const SessionState& state)
{
    recent_ = state.recent;
    setFileDialogRecentPaths(recent_);
    saveFormatOptions_ = state.saveFormatOptions;
    autoSaveRecovery_ = state.autoSaveRecovery;
    autoSaveMinutes_ = state.autoSaveMinutes;
}

void ImportSaveCoordinator::recordCommand(const char* id, const QJsonObject& values)
{
    if (hooks_.recordCommand) {
        hooks_.recordCommand(QString::fromLatin1(id), values);
    }
}

// Before the registry destroys a document, drop the state keyed by its view or
// open token: smart-object edit sessions, recovery snapshots, open signals,
// PDF probes, and per-open source/warning maps.
void ImportSaveCoordinator::onAboutToRemoveDocument(int index)
{
    DocEntry* entry = documents_.docAt(index);
    if (!entry) {
        return;
    }
    for (int i = editSessions_.size() - 1; i >= 0; --i) {
        const SmartObjectEditSession& session = editSessions_.at(i);
        if (session.editor == entry->view || session.origin == entry->view) {
            delete session.temp;
            editSessions_.removeAt(i);
        }
    }
    forgetRecovery(entry->view);
    if (entry->view) {
        openWired_.remove(entry->view);
        pdfProbes_.remove(entry->view);
    }
    openSourcePaths_.remove(entry->token);
    openWarnings_.remove(entry->token);
}

} // namespace pictura
