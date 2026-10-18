// Photoshop PDF (#626, `interop/pdf-export`): Save As / Save through the Save
// Adobe PDF dialog, reopening a Pictura Photoshop PDF, and Edit > Adobe PDF
// Presets. Split from frame_menus.cpp, which sits near the file-size cap.

#include "frame_includes.h"

#include "dialogs.h"
#include "pdf_presets_dialog.h"
#include "pdf_save_dialog.h"
#include "session.h"
#include "type_fonts.h"

#include "pictura_app/src/cxxqt_object/pdf_export.cxxqt.h"

#include <QtCore/QFileInfo>
#include <QtCore/QUrl>
#include <QtGui/QDesktopServices>
#include <QtGui/QGuiApplication>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QStatusBar>

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

bool PicturaMainWindow::isPdfPath(const QString& path)
{
    const QString suffix = QFileInfo(path).suffix().toLower();
    return suffix == QStringLiteral("pdf") || suffix == QStringLiteral("pdp");
}

bool PicturaMainWindow::runPdfDialog(QDialog& dialog)
{
    if (pdfDialogDriver_) {
        return pdfDialogDriver_(dialog);
    }
    if (auto* box = qobject_cast<QMessageBox*>(&dialog)) {
        return box->exec() == QMessageBox::Save;
    }
    return runDialog(dialog, this) == QDialog::Accepted;
}

bool PicturaMainWindow::savePdfWithDialog(const QString& path)
{
    const int index = activeDocumentIndex();
    PictureView* view = viewAt(index);
    if (!view || !view->has_document()) {
        return false;
    }
    // Seed with this document's last PDF settings, else the last used.
    QString seed = pdf_view_options(*view);
    if (seed.isEmpty()) {
        seed = loadSession().pdfOptions;
    }
    PdfSaveDialog dialog(seed, PdfSaveDialog::presetsDirectory(), PdfSaveDialog::Mode::Save,
                         this);
    if (!runPdfDialog(dialog)) {
        return false;
    }
    const QString options = dialog.options();
    if (!isOn(options, "preserveEditing") && documentRequiresLayers(view)) {
        QMessageBox box(QMessageBox::Warning, tr("Save Adobe PDF"),
                        tr("The chosen format cannot hold all of the document's features. "
                           "The flattened composite will be written."),
                        QMessageBox::Save | QMessageBox::Cancel, this);
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
    const QString error = pdf_save(*view, path, options);
    if (!error.isEmpty()) {
        statusBar()->showMessage(tr("Save Adobe PDF: %1").arg(error));
        return false;
    }
    SessionState state = loadSession();
    state.pdfOptions = withoutPasswords(options);
    pictura::saveSession(state);
    docs_[index].path = path;
    dropRecoverySnapshot(view);
    updateTabTitle(index);
    rememberRecent(path);
    updateWindowTitle();
    if (isOn(options, "viewAfterSaving")) {
        if (pdfViewer_) {
            pdfViewer_(path);
        } else if (QGuiApplication::platformName() != QStringLiteral("offscreen")) {
            QDesktopServices::openUrl(QUrl::fromLocalFile(path));
        }
    }
    return true;
}

bool PicturaMainWindow::openPdfPath(const QString& path, bool interactive)
{
    auto* view = new PictureView(this);
    view->set_color_policy(colorPolicy_);
    // A protected PDF asks for its password here, so its editing data opens
    // with its layers; without editing data the import reuses the password.
    QString password;
    for (;;) {
        beginOpenIndicator(path);
        const QString result = pdf_open(*view, path, password);
        endOpenIndicator();
        if (result == QStringLiteral("ok")) {
            addDocument(view, path);
            return true;
        }
        if (result == QStringLiteral("none") || !interactive) {
            delete view;
            return importPdfPath(path, interactive, password);
        }
        if (!askPdfPassword(path, result == QStringLiteral("wrongpassword"), password)) {
            delete view;
            return false;
        }
    }
}

void PicturaMainWindow::showPdfPresets()
{
    PdfPresetsDialog dialog(PdfSaveDialog::presetsDirectory(),
                            [this](QDialog& editor) { return runPdfDialog(editor); }, this);
    runPdfDialog(dialog);
}

void PicturaMainWindow::wirePdfCommands()
{
    registry_->setHandler(command_ids::EditPdfPresets, [this]() { showPdfPresets(); });
}

} // namespace pictura
