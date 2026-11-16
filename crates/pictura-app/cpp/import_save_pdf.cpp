// Photoshop PDF: Save As / Save through the Save Adobe PDF dialog, reopening a
// Pictura Photoshop PDF, and Edit > Adobe PDF Presets.

#include "import_save_coordinator.h"

#include "dialogs.h"
#include "document_registry.h"
#include "pdf_presets_dialog.h"
#include "pdf_save_dialog.h"
#include "session.h"
#include "type_fonts.h"

#include "api/document_api.h"
#include "api/document_bridge.h"
#include "api/engine_bridge.h"
#include "api/proofing_bridge.h"

#include <QtCore/QFileInfo>
#include <QtCore/QUrl>
#include <QtGui/QDesktopServices>
#include <QtGui/QGuiApplication>
#include <QtWidgets/QMessageBox>

namespace pictura {

namespace {

// Passwords never reach the session file.
QString withoutPasswords(const QString& options)
{
    const QString open = pdf_set_option(options, QStringLiteral("openPassword"), QString());
    return pdf_set_option(open, QStringLiteral("permissionsPassword"), QString());
}

bool isOn(const QString& options, const char* field)
{
    return pdf_option(options, QString::fromLatin1(field)) == QStringLiteral("1");
}

} // namespace

bool ImportSaveCoordinator::isPdfPath(const QString& path)
{
    const QString suffix = QFileInfo(path).suffix().toLower();
    return suffix == QStringLiteral("pdf") || suffix == QStringLiteral("pdp");
}

bool ImportSaveCoordinator::runPdfDialog(QDialog& dialog)
{
    if (pdfDialogDriver_) {
        return pdfDialogDriver_(dialog);
    }
    if (auto* box = qobject_cast<QMessageBox*>(&dialog)) {
        return box->exec() == QMessageBox::Save;
    }
    return runDialog(dialog, dialogParent_) == QDialog::Accepted;
}

bool ImportSaveCoordinator::savePdfWithDialog(const QString& path, bool copy)
{
    const int index = documents_.activeIndex();
    PictureView* view = documents_.viewAt(index);
    if (!view || !view->has_document()) {
        return false;
    }
    // Seed with this document's last PDF settings, else the last used.
    QString seed = pdf_view_options(*view);
    if (seed.isEmpty()) {
        seed = loadSession().pdfOptions;
    }
    PdfSaveDialog::SaveContext context;
    context.documentIs16Bit = pdf_document_is_16bit(*view);
    context.documentIsContinuousTone = pdf_document_is_continuous_tone(*view);
    context.documentIsBitmap = pdf_document_is_bitmap(*view);
    context.hasSpotChannels = pdf_spot_channel_count(*view) > 0;
    context.proofSetupName = proof_setup_name(*view);
    PdfSaveDialog dialog(seed, PdfSaveDialog::presetsDirectory(), PdfSaveDialog::Mode::Save,
                         context, dialogParent_);
    if (!runPdfDialog(dialog)) {
        return false;
    }
    const QString options = dialog.options();
    if (!isOn(options, "preserveEditing") && documentRequiresLayers(view)) {
        QMessageBox box(QMessageBox::Warning, tr("Save Adobe PDF"),
                        tr("The chosen format cannot hold all of the document's features. "
                           "The flattened composite will be written."),
                        QMessageBox::Save | QMessageBox::Cancel, dialogParent_);
        box.setObjectName(QStringLiteral("pdfFeaturesWarning"));
        if (!runPdfDialog(box)) {
            return false;
        }
    }
    // Type layers become live text in the face they name, so hand every one to
    // the engine first; a document opened untouched has registered none.
    for (const QString& font : pdf_type_fonts(*view).split(QLatin1Char('\n'), Qt::SkipEmptyParts)) {
        registerTypeFont(familyForFontName(font));
    }
    const QString error = copy ? pdf_save_copy(*view, path, options)
                               : pdf_save(*view, path, options);
    if (!error.isEmpty()) {
        if (hooks_.showStatus) {
            hooks_.showStatus(tr("Save Adobe PDF: %1").arg(error), 0);
        }
        return false;
    }
    SessionState state = loadSession();
    state.pdfOptions = withoutPasswords(options);
    pictura::saveSession(state);
    if (!copy) {
        documents_.setDocumentPath(index, path);
        dropRecoverySnapshot(view);
        documents_.updateTabTitle(index);
        rememberRecent(path);
        if (hooks_.updateWindowTitle) {
            hooks_.updateWindowTitle();
        }
    }
    if (isOn(options, "viewAfterSaving")) {
        if (pdfViewer_) {
            pdfViewer_(path);
        } else if (QGuiApplication::platformName() != QStringLiteral("offscreen")) {
            QDesktopServices::openUrl(QUrl::fromLocalFile(path));
        }
    }
    return true;
}

quint64 ImportSaveCoordinator::startOpenPdfPath(const QString& path, bool interactive)
{
    const quint64 token = nextOpenToken();
    beginOpenCursor();
    cursorTokens_.insert(token);
    startPdfProbe(path, interactive, QString(), token);
    return token;
}

void ImportSaveCoordinator::startPdfProbe(const QString& path, bool interactive,
                                          const QString& password, quint64 token)
{
    auto* view = new PictureView(dialogParent_);
    if (hooks_.applyColorSettings) {
        hooks_.applyColorSettings(view);
    }
    view->set_gpu_compute(hooks_.gpuCompute ? hooks_.gpuCompute() : true);
    open_set_delay(*view, quint64(openDelayForTest_));
    wireOpenSignals(view);
    pdfProbes_.insert(view, PdfProbe{token, path, interactive, password});
    open_pdf_start(*view, path, password);
}

void ImportSaveCoordinator::showPdfPresets()
{
    PdfPresetsDialog dialog(PdfSaveDialog::presetsDirectory(),
                            [this](QDialog& editor) { return runPdfDialog(editor); },
                            dialogParent_);
    runPdfDialog(dialog);
}

} // namespace pictura
