// Generic PDF and EPS import through Ghostscript (`interop/pdf-eps-import`,
// #627): the password prompt, the Import PDF and Rasterize Generic EPS Format
// dialogs, one untitled document per chosen page or image, the warnings box,
// and File > Open As (`interop/open-as`).

#include "frame_includes.h"

#include "dialogs.h"
#include "import_pdf_dialog.h"
#include "raster_size_fields.h"
#include "rasterize_eps_dialog.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/open_jobs.cxxqt.h"
#include "pictura_app/src/cxxqt_object/pdf_export/import.cxxqt.h"

#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QTimer>
#include <QtWidgets/QApplication>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QStatusBar>

#include <memory>

namespace pictura {

namespace {

// Open As's formats, in filter order, and the leading bytes each decoder
// accepts (empty: checked by the decoder itself).
struct OpenAsFormat {
    const char* filter;
    const char* kind; // "psd", "pdf", "eps", or "image"
    QByteArrayList signatures;
};

const QVector<OpenAsFormat>& openAsFormats()
{
    static const QVector<OpenAsFormat> formats = {
        {"Photoshop (*.PSD *.PDD)", "psd", {QByteArray("8BPS")}},
        {"Large Document Format (*.PSB)", "psd", {QByteArray("8BPS")}},
        {"Generic PDF (*.PDF *.PDP *.AI)", "pdf", {}},
        {"Generic EPS (*.EPS *.EPSF *.PS)", "eps", {}},
        {"PNG (*.PNG)", "image", {QByteArray("\x89PNG")}},
        {"JPEG (*.JPG *.JPEG *.JPE)", "image", {QByteArray("\xFF\xD8\xFF")}},
        {"TIFF (*.TIF *.TIFF)",
         "image",
         {QByteArray("II*\0", 4), QByteArray("MM\0*", 4)}},
        {"WebP (*.WEBP)", "image", {QByteArray("RIFF")}},
        {"GIF (*.GIF)", "image", {QByteArray("GIF8")}},
        {"BMP (*.BMP)", "image", {QByteArray("BM")}},
    };
    return formats;
}

// `name` with `-<number>` before its extension: `deck.pdf` -> `deck-3.pdf`.
QString numberedName(const QString& name, int number)
{
    const QFileInfo info(name);
    const QString suffix = info.suffix();
    const QString stem = suffix.isEmpty() ? name : name.left(name.size() - suffix.size() - 1);
    return suffix.isEmpty() ? QStringLiteral("%1-%2").arg(stem).arg(number)
                            : QStringLiteral("%1-%2.%3").arg(stem).arg(number).arg(suffix);
}

ImportTarget targetFrom(const RasterSizeFields& size, QSize pixels, bool antiAlias,
                        const QString& name)
{
    ImportTarget target;
    target.width = pixels.width();
    target.height = pixels.height();
    target.anti_alias = antiAlias;
    target.mode = size.mode();
    target.depth = size.depth();
    target.ppi = size.ppi();
    target.per_cm = size.perCm();
    target.name = name;
    return target;
}

// Split a bridge result into its first line and the rest.
QPair<QString, QString> splitResult(const QString& result)
{
    const qsizetype newline = result.indexOf(QLatin1Char('\n'));
    if (newline < 0) {
        return {result, QString()};
    }
    return {result.left(newline), result.mid(newline + 1)};
}

// The probe and thumbnail work is synchronous; a wait cursor marks it without
// blocking the dialog's own interaction.
class ScopedWaitCursor {
public:
    ScopedWaitCursor() { QApplication::setOverrideCursor(Qt::WaitCursor); }
    ~ScopedWaitCursor() { QApplication::restoreOverrideCursor(); }
    ScopedWaitCursor(const ScopedWaitCursor&) = delete;
    ScopedWaitCursor& operator=(const ScopedWaitCursor&) = delete;
};

} // namespace

const QStringList& openAsFileFilters()
{
    static const QStringList filters = [] {
        QStringList list;
        for (const OpenAsFormat& format : openAsFormats()) {
            list.append(QString::fromLatin1(format.filter));
        }
        return list;
    }();
    return filters;
}

bool PicturaMainWindow::runImportDialog(QDialog& dialog)
{
    if (importDialogDriver_) {
        return importDialogDriver_(dialog);
    }
    if (auto* box = qobject_cast<QMessageBox*>(&dialog)) {
        box->exec();
        return true;
    }
    return runDialog(dialog, this) == QDialog::Accepted;
}

void PicturaMainWindow::reportImportError(const QString& path, const QString& reason)
{
    statusBar()->showMessage(
        tr("Could not open %1: %2").arg(QFileInfo(path).fileName(), reason.trimmed()));
}

bool PicturaMainWindow::askPdfPassword(const QString& path, bool refused, QString& password)
{
    QInputDialog ask(this);
    ask.setObjectName(QStringLiteral("pdfPasswordPrompt"));
    ask.setWindowTitle(tr("Password"));
    ask.setLabelText(refused ? tr("The password is incorrect. Password:")
                             : tr("\"%1\" is protected. Password:").arg(QFileInfo(path).fileName()));
    ask.setTextEchoMode(QLineEdit::Password);
    if (!runImportDialog(ask)) {
        return false;
    }
    password = ask.textValue();
    return true;
}

// The Import PDF flow once `pdf_open` found no editing data. The probe and the
// dialog are synchronous; each chosen page is rendered on a worker with its own
// loading tab, and the flow waits for that page before starting the next.
PicturaMainWindow::OpenOutcome PicturaMainWindow::importPdfPathWithToken(
    const QString& path, bool interactive, const QString& knownPassword, quint64 token)
{
    QString password = knownPassword;
    PdfImportProbe probe;
    for (;;) {
        QString result;
        {
            ScopedWaitCursor wait;
            result = QString(pdf_import_probe(path, password));
        }
        const auto [head, body] = splitResult(result);
        if (head == QStringLiteral("ok")) {
            probe = PdfImportProbe::parse(body);
            break;
        }
        const bool locked = head == QStringLiteral("error password");
        const bool refused = head == QStringLiteral("error wrongpassword");
        if (!interactive || (!locked && !refused)) {
            reportImportError(path, body);
            finishOpen(token, OpenOutcome::Failed, body);
            return OpenOutcome::Failed;
        }
        if (!askPdfPassword(path, refused, password)) {
            finishOpen(token, OpenOutcome::Cancelled);
            return OpenOutcome::Cancelled;
        }
    }
    if (probe.pages.isEmpty()) {
        const QString reason = tr("the PDF has no pages.");
        reportImportError(path, reason);
        finishOpen(token, OpenOutcome::Failed, reason);
        return OpenOutcome::Failed;
    }

    // The dialog renders the page thumbnails as it is built.
    std::unique_ptr<ImportPdfDialog> dialog;
    {
        ScopedWaitCursor wait;
        dialog = std::make_unique<ImportPdfDialog>(path, password, probe, this);
    }
    if (interactive && !runImportDialog(*dialog)) {
        finishOpen(token, OpenOutcome::Cancelled);
        return OpenOutcome::Cancelled;
    }
    const QList<int> chosen = dialog->selection();
    const bool images = dialog->imagesMode();
    const QString baseName = dialog->name();
    const bool suppressWarnings = dialog->suppressWarnings();
    QStringList warnings;
    int opened = 0;
    for (int index : chosen) {
        const QString name =
            chosen.size() == 1 ? baseName : numberedName(baseName, index + 1);
        const QSize pixels = images ? QSize(probe.images.value(index).width,
                                            probe.images.value(index).height)
                                    : dialog->pixelSizeFor(index);
        const ImportTarget target =
            targetFrom(*dialog->sizeFields(), pixels, dialog->antiAliased(), name);
        const QString area = images ? QString() : dialog->areaFor(index);

        const quint64 pageToken = nextOpenToken();
        auto* view = new PictureView(this);
        beginLoadingDocument(view, QString(), name, pageToken, QString(), OpenKind::PdfPage);
        openSourcePaths_.insert(pageToken, path);
        if (images) {
            open_pdf_image_start(*view, path, index, target);
        } else {
            open_pdf_page_start(*view, path, password, index + 1, area, target);
        }
        const OpenOutcome outcome = waitForOpen(pageToken);
        warnings += openWarnings_.take(pageToken);
        if (outcome == OpenOutcome::Cancelled) {
            finishOpen(token, OpenOutcome::Cancelled);
            return OpenOutcome::Cancelled;
        }
        if (outcome != OpenOutcome::Opened) {
            const OpenOutcome result = opened > 0 ? OpenOutcome::Opened : OpenOutcome::Failed;
            finishOpen(token, result);
            return result;
        }
        ++opened;
    }
    if (opened == 0) {
        finishOpen(token, OpenOutcome::Failed);
        return OpenOutcome::Failed;
    }
    rememberRecent(path);
    warnings.removeDuplicates();
    if (interactive && !warnings.isEmpty() && !suppressWarnings) {
        QMessageBox box(QMessageBox::Warning, tr("Import PDF"), warnings.join(QLatin1Char('\n')),
                        QMessageBox::Ok, this);
        box.setObjectName(QStringLiteral("importWarnings"));
        runImportDialog(box);
    }
    finishOpen(token, OpenOutcome::Opened);
    return OpenOutcome::Opened;
}

quint64 PicturaMainWindow::startImportEpsPath(const QString& path, bool interactive)
{
    const quint64 token = nextOpenToken();
    const auto [head, body] = splitResult(QString(eps_import_probe(path)));
    const QStringList box = body.split(QLatin1Char(' '), Qt::SkipEmptyParts);
    if (head != QStringLiteral("ok") || box.size() != 4) {
        const QString reason = tr("format mismatch or damaged file.");
        reportImportError(path, reason);
        QTimer::singleShot(0, this, [this, token, reason] {
            finishOpen(token, OpenOutcome::Failed, reason);
        });
        return token;
    }
    const QSizeF points(box[2].toDouble() - box[0].toDouble(),
                        box[3].toDouble() - box[1].toDouble());
    RasterizeEpsDialog dialog(points, this);
    if (interactive && !runImportDialog(dialog)) {
        QTimer::singleShot(0, this, [this, token] {
            finishOpen(token, OpenOutcome::Cancelled);
        });
        return token;
    }
    const QString name = QFileInfo(path).fileName();
    const ImportTarget target = targetFrom(*dialog.sizeFields(), dialog.sizeFields()->pixelSize(),
                                           dialog.antiAliased(), name);
    beginOpenCursor();
    cursorTokens_.insert(token);
    auto* view = new PictureView(this);
    beginLoadingDocument(view, QString(), name, token, QString(), OpenKind::Eps);
    openSourcePaths_.insert(token, path);
    open_eps_start(*view, path, target);
    return token;
}

void PicturaMainWindow::showOpenAsDialog()
{
    QString selected = openAsFileFilters().first();
    const QString path = getOpenFileName(this, tr("Open As"), documentDirectory(activeFilePath()),
                                         openAsFileFilters(), &selected);
    if (!path.isEmpty()) {
        startOpenAsPath(path, std::max(0, int(openAsFileFilters().indexOf(selected))));
    }
}

quint64 PicturaMainWindow::startOpenAsPath(const QString& path, int formatIndex)
{
    const OpenAsFormat& format =
        openAsFormats().at(std::clamp(formatIndex, 0, int(openAsFormats().size()) - 1));
    QFile file(path);
    const QByteArray head = file.open(QIODevice::ReadOnly) ? file.read(16) : QByteArray();
    const QString kind = QString::fromLatin1(format.kind);
    bool matches = format.signatures.isEmpty();
    for (const QByteArray& signature : format.signatures) {
        matches = matches || head.startsWith(signature);
    }
    if (kind == QStringLiteral("pdf") || kind == QStringLiteral("eps")) {
        matches = QString(vector_sniff(path)) == kind;
    }
    if (!matches) {
        reportImportError(path, tr("format mismatch or damaged file."));
        return 0;
    }
    if (kind == QStringLiteral("pdf")) {
        return startOpenPdfPath(path);
    }
    if (kind == QStringLiteral("eps")) {
        return startImportEpsPath(path);
    }
    const quint64 token = kind == QStringLiteral("psd") ? startOpenPath(path)
                                                       : startOpenImagePath(path);
    if (token != 0) {
        openFailureMessages_.insert(token, tr("format mismatch or damaged file."));
    }
    return token;
}

bool PicturaMainWindow::openAsPath(const QString& path, int formatIndex)
{
    const quint64 token = startOpenAsPath(path, formatIndex);
    return token != 0 && waitForOpen(token) == OpenOutcome::Opened;
}

} // namespace pictura
