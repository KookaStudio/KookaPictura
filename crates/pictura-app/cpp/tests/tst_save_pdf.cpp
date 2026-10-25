// Photoshop PDF save (#626, `interop/pdf-export`): the Save As format, the
// Save Adobe PDF dialog and its gating, presets and the preset manager, the
// window's save/reopen flow, and JPEG through the Qt boundary.

#include <QtTest/QtTest>

#include "api/document_api.h"
#include "api/type_api.h"
#include "dialogs.h"
#include "pdf_presets_dialog.h"
#include "pdf_save_dialog.h"
#include "session.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/pdf_export.cxxqt.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QProcess>
#include <QtCore/QStandardPaths>
#include <QtCore/QTemporaryDir>
#include <QtGui/QImage>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QMessageBox>

#include "qt_test_support.h"

using namespace pictura;

namespace {

using Fields = QList<QPair<QString, QString>>;

// A dialog driver: set `fields` on the Save Adobe PDF dialog and accept it;
// answer a message box with `warningAnswer`, recording its text.
std::function<bool(QDialog&)> driver(const Fields& fields, QString* warning = nullptr,
                                     bool warningAnswer = true)
{
    return [fields, warning, warningAnswer](QDialog& dialog) {
        if (auto* box = qobject_cast<QMessageBox*>(&dialog)) {
            if (warning) {
                *warning = box->text();
            }
            return warningAnswer;
        }
        auto* pdf = qobject_cast<PdfSaveDialog*>(&dialog);
        for (const auto& [field, value] : fields) {
            pdf->setFieldForTest(field, value);
        }
        return true;
    };
}

QByteArray readAll(const QString& path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
}

// A textured RGB image opened as the active document.
PictureView* openGradient(PicturaMainWindow& window, const QTemporaryDir& dir)
{
    QImage image(48, 32, QImage::Format_RGB32);
    for (int y = 0; y < image.height(); ++y) {
        for (int x = 0; x < image.width(); ++x) {
            image.setPixelColor(x, y, QColor(x * 5, y * 7, 255 - x * 3));
        }
    }
    const QString png = dir.filePath(QStringLiteral("gradient.png"));
    if (!image.save(png) || !window.openImagePath(png)) {
        return nullptr;
    }
    return window.activeView();
}

// Mean absolute RGB difference between page 1 of `pdf`, rendered by poppler at
// 72 ppi, and `expected`; -1 when the render fails.
double renderDifference(const QString& pdf, const QImage& expected)
{
    const QString out = pdf + QStringLiteral(".render");
    QProcess::execute(QStringLiteral("pdftoppm"),
                      {QStringLiteral("-singlefile"), QStringLiteral("-r"), QStringLiteral("72"),
                       QStringLiteral("-png"), pdf, out});
    const QImage rendered(out + QStringLiteral(".png"));
    if (rendered.size() != expected.size()) {
        return -1;
    }
    double sum = 0;
    for (int y = 0; y < expected.height(); ++y) {
        for (int x = 0; x < expected.width(); ++x) {
            const QColor a = rendered.pixelColor(x, y);
            const QColor b = expected.pixelColor(x, y);
            sum += qAbs(a.red() - b.red()) + qAbs(a.green() - b.green())
                   + qAbs(a.blue() - b.blue());
        }
    }
    return sum / (3.0 * expected.width() * expected.height());
}

} // namespace

class SavePdfTest : public QObject {
    Q_OBJECT

private slots:
    void saveAsListsPhotoshopPdf();
    void pdfSaveRecordsPath();
    void resaveReopensDialogWithSettings();
    void cancelWritesNothing();
    void failedWriteCreatesNoFile();
    void jpegCompressionRenders();
    void jpeg2000IsDisabled();
    void reopenRestoresLayers();
    void preserveOffWarnsForLayers();
    void viewAfterSavingOpensFile();
    void standardLocksSettings();
    void builtInPresetsListed();
    void editMarksPresetModified();
    void savePresetWritesFile();
    void presetManagerEditsUserPresets();
    void dialogHasFivePages();
    void compatibilityGatesControls();
    void summaryListsSettings();
    void typeLayerIsLiveText();
};

// pdf_save_as_format
void SavePdfTest::saveAsListsPhotoshopPdf()
{
    const QString filter = QStringLiteral("Photoshop PDF (*.PDF *.PDP)");
    const int index = saveFileFilters().indexOf(filter);
    QVERIFY(index >= 0);
    QCOMPARE(defaultSuffixForFilter(index), QStringLiteral(".pdf"));
    QVERIFY(openFileFilters().contains(filter));
    QVERIFY(PicturaMainWindow::isPdfPath(QStringLiteral("poster.PDP")));
    QVERIFY(!PicturaMainWindow::isPdfPath(QStringLiteral("poster.psd")));
    QCOMPARE(lowercasedImageSuffix(QStringLiteral("/tmp/poster.PDF")),
             QStringLiteral("/tmp/poster.pdf"));
}

// pdf_path_becomes_pdf
void SavePdfTest::pdfSaveRecordsPath()
{
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Poster"), 24, 16, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    PictureView* view = window->activeView();
    view->add_layer(0);
    QVERIFY(document_api::isDirty(*view));
    const QString path = dir.filePath(QStringLiteral("poster.pdf"));
    window->setPdfDialogDriverForTest(driver({}));
    QVERIFY(window->savePdfWithDialog(path));
    QVERIFY(readAll(path).startsWith("%PDF-1.5"));
    QCOMPARE(document_api::filePath(*view), path);
    QVERIFY(!document_api::isDirty(*view));
    QVERIFY(!loadSession().pdfOptions.isEmpty());
}

// pdf_resave_shows_dialog
void SavePdfTest::resaveReopensDialogWithSettings()
{
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Poster"), 24, 16, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    const QString path = dir.filePath(QStringLiteral("poster.pdf"));
    window->setPdfDialogDriverForTest(driver({{QStringLiteral("compression"), QStringLiteral("2")}}));
    QVERIFY(window->savePdfWithDialog(path));
    window->activeView()->add_layer(0);

    QString seen;
    window->setPdfDialogDriverForTest([&seen](QDialog& dialog) {
        auto* pdf = qobject_cast<PdfSaveDialog*>(&dialog);
        seen = pdf ? pdf_option(pdf->options(), QStringLiteral("compression")) : QString();
        return false;
    });
    QVERIFY(!window->saveActive());
    QCOMPARE(seen, QStringLiteral("2"));
}

// pdf_cancel_writes_nothing
void SavePdfTest::cancelWritesNothing()
{
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Poster"), 8, 8, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    PictureView* view = window->activeView();
    view->add_layer(0);
    const QString path = dir.filePath(QStringLiteral("cancelled.pdf"));
    window->setPdfDialogDriverForTest([](QDialog&) { return false; });
    QVERIFY(!window->savePdfWithDialog(path));
    QVERIFY(!QFile::exists(path));
    QVERIFY(document_api::filePath(*view).isEmpty());
    QVERIFY(document_api::isDirty(*view));
}

// pdf_atomic_failure
void SavePdfTest::failedWriteCreatesNoFile()
{
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Poster"), 8, 8, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    const QString path = dir.filePath(QStringLiteral("missing/poster.pdf"));
    window->setPdfDialogDriverForTest(driver({}));
    QVERIFY(!window->savePdfWithDialog(path));
    QVERIFY(!QFile::exists(path));
    QVERIFY(!pdf_save(*window->activeView(), path, pdf_default_options()).isEmpty());
}

// pdf_jpeg
void SavePdfTest::jpegCompressionRenders()
{
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    PictureView* view = openGradient(*window, dir);
    QVERIFY(view);
    const QImage source(dir.filePath(QStringLiteral("gradient.png")));
    const auto save = [&](const QString& name, const Fields& fields) {
        const QString path = dir.filePath(name);
        window->setPdfDialogDriverForTest(driver(fields));
        return window->savePdfWithDialog(path) ? path : QString();
    };
    const Fields jpeg = {{QStringLiteral("preserveEditing"), QStringLiteral("0")},
                         {QStringLiteral("embedThumbnails"), QStringLiteral("0")},
                         {QStringLiteral("compression"), QStringLiteral("2")}};
    const QString maximum = save(QStringLiteral("max.pdf"),
                                 jpeg + Fields{{QStringLiteral("quality"), QStringLiteral("4")}});
    const QString low = save(QStringLiteral("low.pdf"),
                             jpeg + Fields{{QStringLiteral("quality"), QStringLiteral("1")}});
    const QString cmyk = save(QStringLiteral("cmyk.pdf"),
                              jpeg + Fields{{QStringLiteral("quality"), QStringLiteral("4")},
                                            {QStringLiteral("colorConversion"), QStringLiteral("1")},
                                            {QStringLiteral("destination"), QStringLiteral("4")},
                                            {QStringLiteral("profileInclusion"), QStringLiteral("1")}});
    QVERIFY(!maximum.isEmpty() && !low.isEmpty() && !cmyk.isEmpty());
    QVERIFY(readAll(maximum).contains("/Filter /DCTDecode"));
    QVERIFY(readAll(cmyk).contains("/Decode [1 0 1 0 1 0 1 0]"));
    QVERIFY(QFileInfo(low).size() < QFileInfo(maximum).size());
    if (QStandardPaths::findExecutable(QStringLiteral("pdftoppm")).isEmpty()) {
        QSKIP("pdftoppm is not installed");
    }
    const double maxDiff = renderDifference(maximum, source);
    QVERIFY2(maxDiff >= 0 && maxDiff < 4, qPrintable(QString::number(maxDiff)));
    const double lowDiff = renderDifference(low, source);
    QVERIFY2(lowDiff >= 0 && lowDiff < 12, qPrintable(QString::number(lowDiff)));
    // A polarity error inverts every colour (a mean far above 100).
    const double cmykDiff = renderDifference(cmyk, source);
    QVERIFY2(cmykDiff >= 0 && cmykDiff < 60, qPrintable(QString::number(cmykDiff)));
}

// pdf_jpeg2000_disabled
void SavePdfTest::jpeg2000IsDisabled()
{
    QTemporaryDir dir;
    PdfSaveDialog dialog(pdf_default_options(), dir.path(), PdfSaveDialog::Mode::Save);
    auto* compression =
        qobject_cast<QComboBox*>(dialog.fieldWidgetForTest(QStringLiteral("compression")));
    QVERIFY(compression);
    QCOMPARE(compression->itemText(3), QStringLiteral("JPEG 2000"));
    QVERIFY(!dialog.compressionItemEnabled(3));
    QVERIFY(dialog.compressionItemEnabled(2));
}

// pdf_reopen_layers
void SavePdfTest::reopenRestoresLayers()
{
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Layers"), 16, 16, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    window->activeView()->add_layer(0);
    const int layers = window->activeView()->layer_count();
    QCOMPARE(layers, 2);
    const QString path = dir.filePath(QStringLiteral("layers.pdf"));
    window->setPdfDialogDriverForTest(driver({}));
    QVERIFY(window->savePdfWithDialog(path));

    QVERIFY(window->openDocumentAtPath(path));
    PictureView* reopened = window->activeView();
    QCOMPARE(reopened->layer_count(), layers);
    QCOMPARE(document_api::filePath(*reopened), path);
    reopened->add_layer(0);
    QVERIFY(window->revertActive());
    QCOMPARE(reopened->layer_count(), layers);
}

// pdf_flat_warns
void SavePdfTest::preserveOffWarnsForLayers()
{
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Layers"), 16, 16, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    window->activeView()->add_layer(0);
    const QString path = dir.filePath(QStringLiteral("flat.pdf"));
    QString warning;
    window->setPdfDialogDriverForTest(
        driver({{QStringLiteral("preserveEditing"), QStringLiteral("0")}}, &warning, false));
    QVERIFY(!window->savePdfWithDialog(path));
    QVERIFY(warning.contains(QStringLiteral("cannot hold all of the document's features")));
    QVERIFY(!QFile::exists(path));

    warning.clear();
    window->setPdfDialogDriverForTest(driver({}, &warning, false));
    QVERIFY(window->savePdfWithDialog(path));
    QVERIFY(warning.isEmpty());
}

// pdf_view_after
void SavePdfTest::viewAfterSavingOpensFile()
{
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("View"), 8, 8, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    QString viewed;
    window->setPdfViewerForTest([&viewed](const QString& path) { viewed = path; });
    const QString path = dir.filePath(QStringLiteral("view.pdf"));
    window->setPdfDialogDriverForTest(driver({}));
    QVERIFY(window->savePdfWithDialog(path));
    QVERIFY(viewed.isEmpty());
    window->setPdfDialogDriverForTest(
        driver({{QStringLiteral("viewAfterSaving"), QStringLiteral("1")}}));
    QVERIFY(window->savePdfWithDialog(path));
    QCOMPARE(viewed, path);
}

// pdf_standard_locks
void SavePdfTest::standardLocksSettings()
{
    QTemporaryDir dir;
    PdfSaveDialog dialog(pdf_default_options(), dir.path(), PdfSaveDialog::Mode::Save);
    // A password field opens only with its "Require a password" box.
    QVERIFY(!dialog.fieldWidgetForTest(QStringLiteral("openPassword"))->isEnabled());
    dialog.setFieldForTest(QStringLiteral("standard"), QStringLiteral("1"));
    auto* compatibility =
        qobject_cast<QComboBox*>(dialog.fieldWidgetForTest(QStringLiteral("compatibility")));
    QCOMPARE(compatibility->currentText(), QStringLiteral("Acrobat 4 (PDF 1.3)"));
    QVERIFY(!compatibility->isEnabled());
    QVERIFY(!dialog.fieldWidgetForTest(QStringLiteral("printing"))->isEnabled());
    QVERIFY(!dialog.fieldWidgetForTest(QStringLiteral("enableCopying"))->isEnabled());
    auto* require = dialog.findChild<QCheckBox*>(QStringLiteral("pdfRequire_openPassword"));
    QVERIFY(require && !require->isEnabled());
}

// pdf_builtin_presets
void SavePdfTest::builtInPresetsListed()
{
    QTemporaryDir dir;
    PdfSaveDialog dialog(pdf_default_options(), dir.path(), PdfSaveDialog::Mode::Save);
    const QStringList expected = {
        QStringLiteral("[Photoshop Default]"),   QStringLiteral("[High Quality Print]"),
        QStringLiteral("[Press Quality]"),       QStringLiteral("[Smallest File Size]"),
        QStringLiteral("[PDF/X-1a:2001]"),       QStringLiteral("[PDF/X-3:2002]"),
        QStringLiteral("[PDF/X-4:2008]"),        QStringLiteral("[PDF/A-1b:2005]"),
        QStringLiteral("[Standard]"),            QStringLiteral("[Rich Content PDF]"),
        QStringLiteral("[Illustrator Default]"), QStringLiteral("[Oversized Pages]"),
    };
    QCOMPARE(dialog.presetNames().mid(0, 12), expected);
    QCOMPARE(dialog.presetLabel(), QStringLiteral("[Photoshop Default]"));
    QVERIFY(dialog.selectPreset(QStringLiteral("[Smallest File Size]")));
    const QString options = dialog.options();
    QCOMPARE(pdf_option(options, QStringLiteral("compression")), QStringLiteral("2"));
    QCOMPARE(pdf_option(options, QStringLiteral("quality")), QStringLiteral("1"));
    QCOMPARE(pdf_option(options, QStringLiteral("downsample")), QStringLiteral("3"));
    QCOMPARE(pdf_option(options, QStringLiteral("downsamplePpi")), QStringLiteral("100"));
}

// pdf_preset_modified
void SavePdfTest::editMarksPresetModified()
{
    QTemporaryDir dir;
    PdfSaveDialog dialog(pdf_default_options(), dir.path(), PdfSaveDialog::Mode::Save);
    QVERIFY(dialog.selectPreset(QStringLiteral("[High Quality Print]")));
    QCOMPARE(dialog.presetLabel(), QStringLiteral("[High Quality Print]"));
    // The Save options are not part of a preset.
    dialog.setFieldForTest(QStringLiteral("notes"), QStringLiteral("0"));
    QCOMPARE(dialog.presetLabel(), QStringLiteral("[High Quality Print]"));
    dialog.setFieldForTest(QStringLiteral("quality"), QStringLiteral("1"));
    QCOMPARE(dialog.presetLabel(), QStringLiteral("[High Quality Print] (Modified)"));
    QVERIFY(dialog.selectPreset(QStringLiteral("[Press Quality]")));
    QVERIFY(dialog.presetNames().contains(QStringLiteral("[High Quality Print]")));
    QCOMPARE(pdf_option(dialog.options(), QStringLiteral("notes")), QStringLiteral("0"));
}

// pdf_save_preset
void SavePdfTest::savePresetWritesFile()
{
    QTemporaryDir dir;
    PdfSaveDialog dialog(pdf_default_options(), dir.path(), PdfSaveDialog::Mode::Save);
    dialog.setFieldForTest(QStringLiteral("quality"), QStringLiteral("2"));
    dialog.setFieldForTest(QStringLiteral("openPassword"), QStringLiteral("secret"));
    QVERIFY(dialog.savePresetAs(QStringLiteral("Web proof")));
    const QString file = dir.filePath(QStringLiteral("Web proof.joboptions"));
    QVERIFY(QFile::exists(file));
    QVERIFY(!readAll(file).contains("secret"));
    QVERIFY(dialog.presetNames().contains(QStringLiteral("Web proof")));
    QCOMPARE(dialog.presetLabel(), QStringLiteral("Web proof"));
    QCOMPARE(PdfSaveDialog::userPresetNames(dir.path()), QStringList{QStringLiteral("Web proof")});
    QVERIFY(!dialog.savePresetAs(QStringLiteral("[Standard]")));
}

// pdf_presets_manager
void SavePdfTest::presetManagerEditsUserPresets()
{
    QTemporaryDir dir;
    QString quality = QStringLiteral("0");
    PdfPresetsDialog manager(dir.path(), [&quality](QDialog& dialog) {
        qobject_cast<PdfSaveDialog*>(&dialog)->setFieldForTest(QStringLiteral("quality"), quality);
        return true;
    });
    QVERIFY(manager.select(QStringLiteral("[Press Quality]")));
    QVERIFY(!manager.canEdit());
    QVERIFY(!manager.editSelected());
    QVERIFY(!manager.deleteSelected());
    QVERIFY(!manager.description().isEmpty());

    QVERIFY(manager.newPreset(QStringLiteral("Mine")));
    const QString file = dir.filePath(QStringLiteral("Mine.joboptions"));
    QVERIFY(QFile::exists(file));
    QVERIFY(manager.presetNames().contains(QStringLiteral("Mine")));
    QVERIFY(manager.select(QStringLiteral("Mine")) && manager.canEdit());
    quality = QStringLiteral("4");
    QVERIFY(manager.editSelected());
    QCOMPARE(pdf_option(PdfSaveDialog::presetOptions(QStringLiteral("Mine"), dir.path()),
                        QStringLiteral("quality")),
             QStringLiteral("4"));

    const QString exported = dir.filePath(QStringLiteral("exported.joboptions.txt"));
    QVERIFY(manager.saveSelectedAs(exported));
    QVERIFY(manager.deleteSelected());
    QVERIFY(!QFile::exists(file));
    QVERIFY(!manager.presetNames().contains(QStringLiteral("Mine")));

    // Load copies a .joboptions file from anywhere into the presets folder.
    QVERIFY(QDir().mkpath(dir.filePath(QStringLiteral("elsewhere"))));
    const QString outside = dir.filePath(QStringLiteral("elsewhere/Incoming.joboptions"));
    QVERIFY(QFile::copy(exported, outside));
    QVERIFY(manager.load(outside));
    QVERIFY(manager.presetNames().contains(QStringLiteral("Incoming")));
    QVERIFY(QFile::exists(dir.filePath(QStringLiteral("Incoming.joboptions"))));
}

// pdf_dialog_pages
void SavePdfTest::dialogHasFivePages()
{
    QTemporaryDir dir;
    PdfSaveDialog dialog(pdf_default_options(), dir.path(), PdfSaveDialog::Mode::Save);
    const QStringList pages = {QStringLiteral("General"), QStringLiteral("Compression"),
                               QStringLiteral("Output"), QStringLiteral("Security"),
                               QStringLiteral("Summary")};
    QCOMPARE(dialog.pageNames(), pages);
    for (const QString& page : pages) {
        dialog.showPage(page);
        QCOMPARE(dialog.currentPage(), page);
    }
    for (const char* field : {"description", "preserveEditing", "embedThumbnails", "fastWebView",
                              "viewAfterSaving", "downsample", "downsamplePpi",
                              "downsampleAbovePpi", "compression", "quality", "tileSize",
                              "convert16To8", "colorConversion", "destination",
                              "profileInclusion", "outputIntentProfile", "outputCondition",
                              "outputConditionIdentifier", "registryName", "openPassword",
                              "permissionsPassword", "printing", "changes", "enableCopying",
                              "enableAccessibility", "plaintextMetadata", "layers", "notes",
                              "alphaChannels", "spotColors", "iccProfile"}) {
        QVERIFY2(dialog.fieldWidgetForTest(QString::fromLatin1(field)), field);
    }
    QVERIFY(!dialog.fieldWidgetForTest(QStringLiteral("spotColors"))->isEnabled());
}

// pdf_compat_gates
void SavePdfTest::compatibilityGatesControls()
{
    QTemporaryDir dir;
    PdfSaveDialog dialog(pdf_default_options(), dir.path(), PdfSaveDialog::Mode::Save);
    auto* convert =
        qobject_cast<QCheckBox*>(dialog.fieldWidgetForTest(QStringLiteral("convert16To8")));
    QVERIFY(convert->isEnabled());
    dialog.setFieldForTest(QStringLiteral("compatibility"), QStringLiteral("0"));
    QVERIFY(convert->isChecked());
    QVERIFY(!convert->isEnabled());
    QVERIFY(dialog.encryptionLevelText().contains(QStringLiteral("40-bit RC4")));
    dialog.setFieldForTest(QStringLiteral("compatibility"), QStringLiteral("4"));
    QVERIFY(dialog.encryptionLevelText().contains(QStringLiteral("256-bit AES")));
}

// pdf_summary
void SavePdfTest::summaryListsSettings()
{
    QTemporaryDir dir;
    PdfSaveDialog dialog(pdf_default_options(), dir.path(), PdfSaveDialog::Mode::Save);
    QVERIFY(dialog.selectPreset(QStringLiteral("[PDF/X-1a:2001]")));
    const QString summary = dialog.summaryText();
    for (const char* line :
         {"Standard: PDF/X-1a:2001", "Compatibility: Acrobat 4 (PDF 1.3)", "Compression: JPEG",
          "Color Conversion: Convert To Destination (Photocraft Coated CMYK)",
          "Warning: transparency is flattened"}) {
        QVERIFY2(summary.contains(QString::fromLatin1(line)), line);
    }
}

int main(int argc, char *argv[])
{
    return pictura::test::shardedMain<SavePdfTest>(argc, argv);
}
// pdf_live_text_app
void SavePdfTest::typeLayerIsLiveText()
{
    if (QStandardPaths::findExecutable(QStringLiteral("pdftotext")).isEmpty()) {
        QSKIP("pdftotext is not installed");
    }
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Poster"), 320, 120, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    PictureView* view = window->activeView();
    TypeSetting setting{};
    setting.x = 16;
    setting.y = 70;
    setting.xx = 1;
    setting.yy = 1;
    CharacterSetting character = type_api::defaultCharacterSetting();
    character.size = 40;
    const QString font = QStringLiteral("Liberation Sans");
    QVERIFY(!type_api::commitLayer(*view, {QStringLiteral("Kooka Pictura"), font, setting, character, type_api::defaultParagraphSetting()})
                 .isEmpty());
    QCOMPARE(pdf_type_fonts(*view), font);
    const QString path = dir.filePath(QStringLiteral("type.pdf"));
    // Acrobat 7 also lets a CFF face stand in where Qt substitutes one.
    window->setPdfDialogDriverForTest(driver({{QStringLiteral("compatibility"), QStringLiteral("3")}}));
    QVERIFY(window->savePdfWithDialog(path));
    const QByteArray pdf = readAll(path);
    QVERIFY(pdf.contains("/Subtype /Type0") || pdf.contains("/Subtype/Type0"));
    QVERIFY(pdf.contains("/FontFile"));
    QProcess extract;
    extract.start(QStringLiteral("pdftotext"), {path, QStringLiteral("-")});
    QVERIFY(extract.waitForFinished());
    QVERIFY2(QString::fromUtf8(extract.readAllStandardOutput()).contains(QStringLiteral("Kooka Pictura")),
             "the type reads back as text");
}

#include "tst_save_pdf.moc"
