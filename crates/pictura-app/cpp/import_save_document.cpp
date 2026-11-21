// Revert, document close, and smart-object edit sessions.
#include "import_save_coordinator.h"

#include "api/document_api.h"
#include "api/document_bridge.h"
#include "api/engine_bridge.h"
#include "api/layers_bridge.h"
#include "dialogs.h"
#include "document_command.h"
#include "document_registry.h"
#include "session.h"

#include <QtCore/QTemporaryDir>
#include <QtCore/QUrl>
#include <QtGui/QDesktopServices>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

bool ImportSaveCoordinator::revertActive()
{
    const int index = documents_.activeIndex();
    PictureView* view = documents_.viewAt(index);
    if (!view) {
        return false;
    }
    if (backgroundSaves_.contains(view)) {
        return false;
    }
    const QString path = document_api::filePath(*view);
    if (path.isEmpty()) {
        return false;
    }
    // Reload in place through the Rust edge: decode first, preserve history and
    // file identity, then append one undoable Revert state.
    const bool reloaded = document_api::revert(*view, path);
    if (!reloaded) {
        return false;
    }
    acknowledgeSaveResolution(view);
    dropRecoverySnapshot(view);
    lastSaveKinds_.remove(view);
    documents_.updateTabTitle(index);
    if (hooks_.requestRefresh) {
        hooks_.requestRefresh();
    }
    if (!view->open_notice().isEmpty() && hooks_.showStatus) {
        hooks_.showStatus(view->open_notice(), 0);
    }
    return true;
}

bool ImportSaveCoordinator::saveAllDocumentsForRestart()
{
    for (int i = 0; i < documents_.count(); ++i) {
        PictureView* view = documents_.viewAt(i);
        resolvePendingSave(view);
        if (!documents_.isDocumentDirty(i) && !saveResolutionNeedsPrompt(view)) {
            continue;
        }
        switch (askUnsaved(dialogParent_, documents_.documentName(i))) {
        case UnsavedChoice::Cancel:
            return false;
        case UnsavedChoice::Save:
            documents_.setActiveIndex(i);
            if (!saveActive() || documents_.isDocumentDirty(i)) {
                return false;
            }
            break;
        case UnsavedChoice::Discard:
            break;
        }
    }
    return true;
}

bool ImportSaveCoordinator::closeDocument(int index, bool interactive)
{
    if (index < 0 || index >= documents_.count()) {
        return false;
    }
    if (documents_.isLoading(index)) {
        cancelOpen(index);
        return true;
    }
    PictureView* view = documents_.viewAt(index);
    resolvePendingSave(view);
    if (interactive && (documents_.isDocumentDirty(index) || saveResolutionNeedsPrompt(view))) {
        switch (askUnsaved(dialogParent_, documents_.documentName(index))) {
        case UnsavedChoice::Cancel:
            return false;
        case UnsavedChoice::Save:
            documents_.setActiveIndex(index);
            if (!saveActive() || documents_.isDocumentDirty(index)) {
                return false;
            }
            break;
        case UnsavedChoice::Discard:
            break;
        }
    }
    lastSaveKinds_.remove(documents_.viewAt(index));
    documents_.removeDocument(index);
    return true;
}

bool ImportSaveCoordinator::closeActiveDocument(bool interactive)
{
    return closeDocument(documents_.activeIndex(), interactive);
}

bool ImportSaveCoordinator::editSmartObjectContents(const QString& layerPath)
{
    PictureView* origin = documents_.activeView();
    if (!origin || layerPath.isEmpty()
        || !origin->layer_can_edit_smart_object_contents(layerPath)) {
        return false;
    }
    const QString suffix = vector_smart_object_suffix(*origin, layerPath);
    if (!suffix.isEmpty()) {
        return editVectorContents(*origin, layerPath, suffix);
    }
    auto* temp = new QTemporaryDir();
    if (!temp->isValid()) {
        delete temp;
        return false;
    }
    const QString filename = QStringLiteral("contents.psd");
    const QString file = temp->filePath(filename);
    if (!origin->export_smart_object_contents(layerPath, file)) {
        delete temp;
        return false;
    }
    auto* editor = new PictureView(dialogParent_);
    if (!document_api::open(*editor, file)) {
        delete editor;
        delete temp;
        return false;
    }
    documents_.addDocument(editor, QString());
    editSessions_.append(SmartObjectEditSession{editor, origin, layerPath, filename, temp});
    return true;
}

bool ImportSaveCoordinator::editVectorContents(PictureView& origin, const QString& layerPath,
                                                const QString& suffix)
{
    const QString id = vector_smart_object_id(origin, layerPath);
    const auto revision = vector_edit_revision(origin, id);
    const auto generation = origin.document_generation();
    const auto temp = std::make_shared<QTemporaryDir>();
    const QString source = temp->filePath(QStringLiteral("contents.") + suffix);
    if (id.isEmpty() || revision == 0 || !temp->isValid()
        || !origin.export_smart_object_contents(layerPath, source)) {
        QMessageBox::warning(dialogParent_, tr("Edit Contents"),
                             tr("Could not prepare the artwork for editing."));
        return false;
    }
    const bool launched = vectorEditorLauncher_
        ? vectorEditorLauncher_(source)
        : QDesktopServices::openUrl(QUrl::fromLocalFile(source));
    if (!launched) {
        temp->remove();
        QMessageBox::warning(dialogParent_, tr("Edit Contents"),
                             tr("Could not open the artwork in the associated application."));
        return false;
    }
    auto* dialog = new QDialog(dialogParent_);
    dialog->setObjectName(QStringLiteral("vectorEditContents"));
    dialog->setWindowTitle(tr("Edit Contents"));
    dialog->setAttribute(Qt::WA_DeleteOnClose);
    auto* layout = new QVBoxLayout(dialog);
    auto* label = new QLabel(tr("Save the artwork in the external editor, then select Apply."),
                             dialog);
    label->setWordWrap(true);
    layout->addWidget(label);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Apply | QDialogButtonBox::Discard,
                                         dialog);
    layout->addWidget(buttons);
    auto* apply = buttons->button(QDialogButtonBox::Apply);
    auto* discard = buttons->button(QDialogButtonBox::Discard);
    apply->setObjectName(QStringLiteral("vectorEditApply"));
    discard->setObjectName(QStringLiteral("vectorEditDiscard"));
    connect(discard, &QPushButton::clicked, dialog, &QDialog::reject);
    connect(dialog, &QDialog::finished, dialog, [temp]() { temp->remove(); });

    const QPointer<PictureView> target(&origin);
    const QPointer<QDialog> sessionDialog(dialog);
    const QPointer<QPushButton> applyButton(apply);
    const QPointer<QWidget> progressParent(dialogParent_);
    connect(apply, &QPushButton::clicked, dialog,
            [target, sessionDialog, applyButton, progressParent, id, source, generation,
             expected = revision]() mutable {
        if (!target || target->document_generation() != generation) {
            QMessageBox::warning(sessionDialog, tr("Edit Contents"),
                                 tr("The original document is no longer available."));
            return;
        }
        applyButton->setEnabled(false);
        bool closed = false;
        const auto onClose = QObject::connect(
            sessionDialog.data(), &QDialog::finished, target.data(), [&closed, target](int) {
                closed = true;
                document_job_cancel(*target);
            });
        const bool applied = runSourceCommand(progressParent, *target, tr("Edit Contents"),
                                               [&](PictureView& view) {
            return start_vector_edit(view, id, expected, source);
        });
        QObject::disconnect(onClose);
        if (closed || !target || !sessionDialog) {
            return;
        }
        if (applyButton) {
            applyButton->setEnabled(true);
        }
        if (applied) {
            expected = vector_edit_revision(*target, id);
        } else {
            QMessageBox::warning(sessionDialog, tr("Edit Contents"),
                                 tr("Could not apply the edited artwork. Check that the source "
                                    "still exists and contains the selected page."));
        }
    });
    for (int i = vectorEditSessions_.size() - 1; i >= 0; --i) {
        if (!vectorEditSessions_.at(i).dialog) {
            vectorEditSessions_.removeAt(i);
        }
    }
    vectorEditSessions_.append(VectorEditSession{target, sessionDialog});
    dialog->show();
    return true;
}

} // namespace pictura
