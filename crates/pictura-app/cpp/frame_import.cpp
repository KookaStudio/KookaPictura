// Generic PDF and EPS import through Ghostscript (`interop/pdf-eps-import`,
// #627): the password prompt, the Import PDF and Rasterize Generic EPS Format
// dialogs, one untitled document per chosen page or image, the warnings box,
// and File > Open As (`interop/open-as`).

#include "frame_includes.h"

#include "dialogs.h"
#include "import_pdf_dialog.h"
#include "raster_size_fields.h"
#include "rasterize_eps_dialog.h"

#include "pictura_app/src/cxxqt_object/pdf_export/import.cxxqt.h"

#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QStatusBar>

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

// The Import PDF flow once `pdf_open` found no Pictura editing data.
bool PicturaMainWindow::importPdfPath(const QString& path, bool interactive)
{
    QString password;
    PdfImportProbe probe;
    for (;;) {
        beginOpenIndicator(path);
        const auto [head, body] = splitResult(QString(pdf_import_probe(path, password)));
        endOpenIndicator();
        if (head == QStringLiteral("ok")) {
            probe = PdfImportProbe::parse(body);
            break;
        }
        const bool locked = head == QStringLiteral("error password");
        const bool refused = head == QStringLiteral("error wrongpassword");
        if (!interactive || (!locked && !refused)) {
            reportImportError(path, body);
            return false;
        }
        QInputDialog ask(this);
        ask.setObjectName(QStringLiteral("pdfPasswordPrompt"));
        ask.setWindowTitle(tr("Password"));
        ask.setLabelText(locked ? tr("\"%1\" is protected. Password:")
                                      .arg(QFileInfo(path).fileName())
                                : tr("The password is incorrect. Password:"));
        ask.setTextEchoMode(QLineEdit::Password);
        if (!runImportDialog(ask)) {
            return false;
        }
        password = ask.textValue();
    }
    if (probe.pages.isEmpty()) {
        reportImportError(path, tr("the PDF has no pages."));
        return false;
    }

    // The dialog renders the page thumbnails as it is built.
    beginOpenIndicator(path);
    ImportPdfDialog dialog(path, password, probe, this);
    endOpenIndicator();
    if (interactive && !runImportDialog(dialog)) {
        return false;
    }
    const QList<int> chosen = dialog.selection();
    const bool images = dialog.imagesMode();
    QStringList warnings;
    int opened = 0;
    beginOpenIndicator(path);
    for (int index : chosen) {
        const QString name =
            chosen.size() == 1 ? dialog.name() : numberedName(dialog.name(), index + 1);
        const QSize pixels = images ? QSize(probe.images.value(index).width,
                                            probe.images.value(index).height)
                                    : dialog.pixelSizeFor(index);
        const ImportTarget target =
            targetFrom(*dialog.sizeFields(), pixels, dialog.antiAliased(), name);
        auto* view = new PictureView(this);
        view->set_color_policy(colorPolicy_);
        const QString result =
            images ? QString(pdf_import_open_image(*view, path, index, target))
                   : QString(pdf_import_open_page(*view, path, password, index + 1,
                                                  dialog.areaFor(index), target));
        const auto [head, body] = splitResult(result);
        if (head != QStringLiteral("ok")) {
            delete view;
            endOpenIndicator();
            reportImportError(path, body);
            return opened > 0;
        }
        warnings += body.split(QLatin1Char('\n'), Qt::SkipEmptyParts);
        const int tab = addDocument(view, QString());
        if (tab >= 0) {
            docs_[tab].displayName = name;
            updateTabTitle(tab);
        }
        ++opened;
    }
    endOpenIndicator();
    if (opened == 0) {
        return false;
    }
    rememberRecent(path);
    warnings.removeDuplicates();
    if (interactive && !warnings.isEmpty() && !dialog.suppressWarnings()) {
        QMessageBox box(QMessageBox::Warning, tr("Import PDF"), warnings.join(QLatin1Char('\n')),
                        QMessageBox::Ok, this);
        box.setObjectName(QStringLiteral("importWarnings"));
        runImportDialog(box);
    }
    return true;
}

bool PicturaMainWindow::importEpsPath(const QString& path, bool interactive)
{
    const auto [head, body] = splitResult(QString(eps_import_probe(path)));
    const QStringList box = body.split(QLatin1Char(' '), Qt::SkipEmptyParts);
    if (head != QStringLiteral("ok") || box.size() != 4) {
        reportImportError(path, tr("format mismatch or damaged file."));
        return false;
    }
    const QSizeF points(box[2].toDouble() - box[0].toDouble(),
                        box[3].toDouble() - box[1].toDouble());
    RasterizeEpsDialog dialog(points, this);
    if (interactive && !runImportDialog(dialog)) {
        return false;
    }
    const QString name = QFileInfo(path).fileName();
    const ImportTarget target = targetFrom(*dialog.sizeFields(), dialog.sizeFields()->pixelSize(),
                                           dialog.antiAliased(), name);
    auto* view = new PictureView(this);
    view->set_color_policy(colorPolicy_);
    beginOpenIndicator(path);
    const auto [status, detail] = splitResult(QString(eps_import_open(*view, path, target)));
    endOpenIndicator();
    if (status != QStringLiteral("ok")) {
        delete view;
        reportImportError(path, detail);
        return false;
    }
    const int tab = addDocument(view, QString());
    if (tab >= 0) {
        docs_[tab].displayName = name;
        updateTabTitle(tab);
    }
    rememberRecent(path);
    return true;
}

void PicturaMainWindow::showOpenAsDialog()
{
    QString selected = openAsFileFilters().first();
    const QString path = getOpenFileName(this, tr("Open As"), documentDirectory(activeFilePath()),
                                         openAsFileFilters(), &selected);
    if (!path.isEmpty()) {
        openAsPath(path, std::max(0, int(openAsFileFilters().indexOf(selected))));
    }
}

bool PicturaMainWindow::openAsPath(const QString& path, int formatIndex)
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
    const auto mismatch = [&]() {
        reportImportError(path, tr("format mismatch or damaged file."));
        return false;
    };
    if (!matches) {
        return mismatch();
    }
    if (kind == QStringLiteral("pdf")) {
        return openPdfPath(path);
    }
    if (kind == QStringLiteral("eps")) {
        return importEpsPath(path);
    }
    const bool opened = kind == QStringLiteral("psd") ? openPath(path) : openImagePath(path);
    return opened || mismatch();
}

} // namespace pictura
