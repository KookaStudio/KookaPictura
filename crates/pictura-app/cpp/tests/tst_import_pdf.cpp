// Generic PDF / EPS import and File > Open As (#627, `interop/pdf-eps-import`,
// `interop/open-as`): the Import PDF and Rasterize Generic EPS Format dialogs,
// the password prompt, the warnings box, and decoder selection. Cases that
// rasterize need Ghostscript (`gs`) and skip without it; CI installs it.

#include <QtTest/QtTest>

#include "api/document_api.h"
#include "commands.h"
#include "dialogs.h"
#include "image_size_dialog.h"
#include "import_pdf_dialog.h"
#include "pdf_save_dialog.h"
#include "rasterize_eps_dialog.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_mode.cxxqt.h"

#include <QtCore/QFile>
#include <QtCore/QProcess>
#include <QtCore/QStandardPaths>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QImage>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QStatusBar>

#include "qt_test_support.h"

using namespace pictura;

namespace {

using Fields = QList<QPair<QString, QString>>;

// What a driver saw while the import ran.
struct Seen {
    int pdfDialogs = 0;
    int epsDialogs = 0;
    int prompts = 0;
    int warnings = 0;
    int documentsAtDialog = -1;
    QHash<QString, QString> fields;
};

// Answer the import dialogs: apply `fields` to the Import PDF or EPS dialog,
// record `read` fields, answer each password prompt with the next of
// `passwords` (cancel when they run out), and acknowledge warnings.
std::function<bool(QDialog&)> driver(PicturaMainWindow* window, Seen* seen,
                                     const Fields& fields = {}, const QStringList& read = {},
                                     QStringList passwords = {}, bool accept = true)
{
    return [=](QDialog& dialog) mutable {
        if (auto* ask = qobject_cast<QInputDialog*>(&dialog)) {
            ++seen->prompts;
            if (passwords.isEmpty()) {
                return false;
            }
            ask->setTextValue(passwords.takeFirst());
            return true;
        }
        if (qobject_cast<QMessageBox*>(&dialog)) {
            ++seen->warnings;
            return true;
        }
        seen->documentsAtDialog = window->documentCount();
        if (auto* pdf = qobject_cast<ImportPdfDialog*>(&dialog)) {
            ++seen->pdfDialogs;
            for (const auto& [name, value] : fields) {
                pdf->setFieldForTest(name, value);
            }
            for (const QString& name : read) {
                seen->fields.insert(name, pdf->fieldForTest(name));
            }
        } else if (auto* eps = qobject_cast<RasterizeEpsDialog*>(&dialog)) {
            ++seen->epsDialogs;
            for (const auto& [name, value] : fields) {
                eps->setFieldForTest(name, value);
            }
            for (const QString& name : read) {
                seen->fields.insert(name, eps->fieldForTest(name));
            }
        }
        return accept;
    };
}

bool haveGhostscript()
{
    return !QStandardPaths::findExecutable(QStringLiteral("gs")).isEmpty();
}

#define REQUIRE_GS()                                                                               \
    if (!haveGhostscript()) {                                                                      \
        QSKIP("Ghostscript (gs) is not installed");                                                \
    }

// A PDF whose object i + 1 has body `objects[i]`; object 1 is the catalog.
QByteArray pdf(const QList<QByteArray>& objects)
{
    QByteArray out("%PDF-1.7\n");
    QList<qsizetype> offsets;
    for (int i = 0; i < objects.size(); ++i) {
        offsets.append(out.size());
        out += QByteArray::number(i + 1) + " 0 obj\n" + objects[i] + "\nendobj\n";
    }
    const qsizetype xref = out.size();
    out += "xref\n0 " + QByteArray::number(objects.size() + 1) + "\n0000000000 65535 f \n";
    for (qsizetype offset : offsets) {
        out += QByteArray::number(offset).rightJustified(10, '0') + " 00000 n \n";
    }
    out += "trailer\n<< /Size " + QByteArray::number(objects.size() + 1)
           + " /Root 1 0 R >>\nstartxref\n" + QByteArray::number(xref) + "\n%%EOF\n";
    return out;
}

QByteArray stream(const QByteArray& dict, const QByteArray& data)
{
    return "<< " + dict + " /Length " + QByteArray::number(data.size()) + " >>\nstream\n" + data
           + "\nendstream";
}

// A PDF with one page per `(MediaBox, content)` entry.
QByteArray pages(const QList<QPair<QByteArray, QByteArray>>& list)
{
    QList<QByteArray> objects = {"<< /Type /Catalog /Pages 2 0 R >>", QByteArray()};
    QByteArray kids;
    for (const auto& [media, content] : list) {
        const int page = int(objects.size()) + 1;
        kids += QByteArray::number(page) + " 0 R ";
        objects.append("<< /Type /Page /Parent 2 0 R /MediaBox [" + media + "] /Contents "
                       + QByteArray::number(page + 1) + " 0 R >>");
        objects.append(stream(QByteArray(), content));
    }
    objects[1] = "<< /Type /Pages /Kids [" + kids + "] /Count "
                 + QByteArray::number(list.size()) + " >>";
    return pdf(objects);
}

const QByteArray kRedRect("1 0 0 rg 20 20 60 30 re f");

QByteArray redPage()
{
    return pages({{"0 0 144 72", kRedRect}});
}

const QByteArray kBlueEps("%!PS-Adobe-3.0 EPSF-3.0\n"
                          "%%BoundingBox: 0 0 144 72\n"
                          "%%EndComments\n"
                          "0 0 1 setrgbcolor 0 0 144 72 rectfill\n"
                          "showpage\n%%EOF\n");

QString write(const QTemporaryDir& dir, const QString& name, const QByteArray& bytes)
{
    const QString path = dir.filePath(name);
    QFile file(path);
    if (file.open(QIODevice::WriteOnly)) {
        file.write(bytes);
    }
    return path;
}

// The 40×30 RGB samples of the images fixture.
QByteArray imageSamples()
{
    QByteArray samples(40 * 30 * 3, '\0');
    for (int i = 0; i < samples.size(); ++i) {
        samples[i] = char(i * 7 % 251);
    }
    return samples;
}

// A page drawing a 40×30 RGB image twice (one object) and a 16×16 gray image
// in a Form XObject.
QByteArray imagesPdf()
{
    QByteArray gray(256, '\0');
    for (int i = 0; i < 256; ++i) {
        gray[i] = char(i);
    }
    return pdf({
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 /Resources << /XObject << /Im0 5 0 R /Im1 5 0 R "
        "/Fm0 6 0 R >> >> >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R >>",
        stream(QByteArray(),
               "q 40 0 0 30 0 0 cm /Im0 Do Q q 40 0 0 30 50 50 cm /Im1 Do Q /Fm0 Do"),
        stream("/Type /XObject /Subtype /Image /Width 40 /Height 30 /ColorSpace /DeviceRGB "
               "/BitsPerComponent 8",
               imageSamples()),
        stream("/Type /XObject /Subtype /Form /BBox [0 0 200 200] /Resources << /XObject << "
               "/Im2 7 0 R >> >>",
               "q 16 0 0 16 100 100 cm /Im2 Do Q"),
        stream("/Type /XObject /Subtype /Image /Width 16 /Height 16 /ColorSpace /DeviceGray "
               "/BitsPerComponent 8",
               gray),
    });
}

QColor pixel(PictureView* view, int x, int y)
{
    return view->image().pixelColor(x, y);
}

QString status(PicturaMainWindow& window)
{
    return window.statusBar()->currentMessage();
}

} // namespace

class ImportPdfTest : public QObject {
    Q_OBJECT

private slots:
    void genericPdfShowsDialog();
    void picturaPdfSkipsDialog();
    void protectedPicturaPdfKeepsLayers();
    void permissionsOnlyPdfOpensWithoutPrompt();
    void pdfRecognizedBySignature();
    void cancelOpensNothing();
    void dialogDefaults();
    void thumbnailSizeChangesGrid();
    void boundingBoxSetsSize();
    void constrainKeepsAspect();
    void resolutionKeepsPhysicalSize();
    void multiPageShowsLargestBox();
    void opensPageOnTransparentLayer();
    void modeAndDepthApply();
    void multiSelectOpensDocuments();
    void imagesModeListsImages();
    void imagesModeOpensImage();
    void passwordPromptOpens();
    void wrongPasswordAsksAgain();
    void suppressWarningsHidesMessage();
    void missingGhostscriptReports();
    void epsDialogDefaults();
    void epsOpensAtSize();
    void epsCancelOpensNothing();
    void openAsIsLive();
    void openAsForcesFormat();
    void openAsReportsMismatch();
    void openAsReportsCorruptDecode();
    void openFiltersListPdfEps();
    void pdfEntryPointUsesLoadingTab();
    void epsEntryPointUsesLoadingTab();
};

// pdf_import_dialog_shown
void ImportPdfTest::genericPdfShowsDialog()
{
    REQUIRE_GS();
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    Seen seen;
    window->setImportDialogDriverForTest(driver(window.get(), &seen));
    QVERIFY(window->openDocumentAtPath(write(dir, QStringLiteral("sample.pdf"), redPage())));
    QCOMPARE(seen.pdfDialogs, 1);
    QCOMPARE(seen.documentsAtDialog, 0);
    QCOMPARE(window->documentCount(), 1);
}

// pdf_pictura_skips_dialog
void ImportPdfTest::picturaPdfSkipsDialog()
{
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    QImage image(16, 12, QImage::Format_RGB32);
    image.fill(QColor(30, 140, 220));
    const QString png = dir.filePath(QStringLiteral("blue.png"));
    QVERIFY(image.save(png));
    QVERIFY(window->openImagePath(png));
    window->activeView()->add_layer(0);
    const QString path = dir.filePath(QStringLiteral("layers.pdf"));
    window->setPdfDialogDriverForTest([](QDialog&) { return true; });
    QVERIFY(window->savePdfWithDialog(path));
    Seen seen;
    window->setImportDialogDriverForTest(driver(window.get(), &seen));
    QVERIFY(window->openDocumentAtPath(path));
    PictureView* view = window->activeView();
    QCOMPARE(view->layer_count(), 2);
    QCOMPARE(seen.pdfDialogs, 0);
    // The canvas shows the reopened pixels, not a blank frame.
    QCOMPARE(pixel(view, 5, 5).rgba(), QColor(30, 140, 220).rgba());
}

namespace {

// A blue two-layer document saved as a Pictura Photoshop PDF with the given
// passwords (empty: none).
QString saveLayeredPdf(PicturaMainWindow* window, const QTemporaryDir& dir, const QString& open,
                       const QString& permissions)
{
    QImage image(16, 12, QImage::Format_RGB32);
    image.fill(QColor(30, 140, 220));
    const QString png = dir.filePath(QStringLiteral("blue.png"));
    if (!image.save(png) || !window->openImagePath(png)) {
        return {};
    }
    window->activeView()->add_layer(0);
    const QString path = dir.filePath(QStringLiteral("protected.pdf"));
    window->setPdfDialogDriverForTest([=](QDialog& dialog) {
        if (auto* save = qobject_cast<PdfSaveDialog*>(&dialog)) {
            save->setFieldForTest(QStringLiteral("openPassword"), open);
            save->setFieldForTest(QStringLiteral("permissionsPassword"), permissions);
        }
        return true;
    });
    if (!window->savePdfWithDialog(path)) {
        return {};
    }
    window->closeDocument(window->activeDocumentIndex(), false);
    return path;
}

} // namespace

// pdf_encrypted_layers, pdf_encrypted_revert
void ImportPdfTest::protectedPicturaPdfKeepsLayers()
{
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    const QString path =
        saveLayeredPdf(window.get(), dir, QStringLiteral("kooka"), QStringLiteral("owner"));
    QVERIFY(!path.isEmpty());
    QFile file(path);
    QVERIFY(file.open(QIODevice::ReadOnly));
    QVERIFY(file.readAll().contains("/Encrypt"));
    Seen seen;
    window->setImportDialogDriverForTest(
        driver(window.get(), &seen, {}, {}, {QStringLiteral("wrong"), QStringLiteral("kooka")}));
    QVERIFY(window->openDocumentAtPath(path));
    QCOMPARE(seen.prompts, 2);
    QCOMPARE(seen.pdfDialogs, 0);
    PictureView* view = window->activeView();
    QCOMPARE(view->layer_count(), 2);
    QCOMPARE(pixel(view, 5, 5).rgba(), QColor(30, 140, 220).rgba());
    // Revert reopens with the password the document was opened with.
    QVERIFY(view->remove_layer(1));
    QVERIFY(window->revertActive());
    QCOMPARE(window->activeView()->layer_count(), 2);
    QCOMPARE(seen.prompts, 2);
}

// pdf_permissions_only_layers
void ImportPdfTest::permissionsOnlyPdfOpensWithoutPrompt()
{
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    const QString path = saveLayeredPdf(window.get(), dir, QString(), QStringLiteral("owner"));
    QVERIFY(!path.isEmpty());
    Seen seen;
    window->setImportDialogDriverForTest(driver(window.get(), &seen));
    QVERIFY(window->openDocumentAtPath(path));
    QCOMPARE(seen.prompts, 0);
    QCOMPARE(seen.pdfDialogs, 0);
    QCOMPARE(window->activeView()->layer_count(), 2);
}

// pdf_sniff_signature
void ImportPdfTest::pdfRecognizedBySignature()
{
    REQUIRE_GS();
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    Seen seen;
    window->setImportDialogDriverForTest(driver(window.get(), &seen, {}, {}, {}, false));
    QVERIFY(!window->openDocumentAtPath(write(dir, QStringLiteral("art.ai"), redPage())));
    QCOMPARE(seen.pdfDialogs, 1);
}

// pdf_cancel
void ImportPdfTest::cancelOpensNothing()
{
    REQUIRE_GS();
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    Seen seen;
    window->setImportDialogDriverForTest(driver(window.get(), &seen, {}, {}, {}, false));
    QVERIFY(!window->openDocumentAtPath(write(dir, QStringLiteral("sample.pdf"), redPage())));
    QCOMPARE(seen.pdfDialogs, 1);
    QCOMPARE(window->documentCount(), 0);
}

// pdf_dialog_defaults
void ImportPdfTest::dialogDefaults()
{
    REQUIRE_GS();
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    Seen seen;
    const QStringList read = {
        QStringLiteral("pagesChecked"), QStringLiteral("threeDEnabled"),
        QStringLiteral("selectedLabel"), QStringLiteral("name"),
        QStringLiteral("cropTo"), QStringLiteral("antiAliased"),
        QStringLiteral("widthUnit"), QStringLiteral("heightUnit"),
        QStringLiteral("resolution"), QStringLiteral("resolutionUnit"),
        QStringLiteral("mode"), QStringLiteral("depth"),
        QStringLiteral("thumbnailSize"), QStringLiteral("suppress"),
        QStringLiteral("constrain"),
    };
    window->setImportDialogDriverForTest(driver(window.get(), &seen, {}, read, {}, false));
    window->openDocumentAtPath(write(dir, QStringLiteral("sample.pdf"), redPage()));
    QCOMPARE(seen.fields.value(QStringLiteral("pagesChecked")), QStringLiteral("1"));
    QCOMPARE(seen.fields.value(QStringLiteral("threeDEnabled")), QStringLiteral("0"));
    QCOMPARE(seen.fields.value(QStringLiteral("selectedLabel")),
             QStringLiteral("1 of 1 page(s) selected"));
    QCOMPARE(seen.fields.value(QStringLiteral("name")), QStringLiteral("sample.pdf"));
    QCOMPARE(seen.fields.value(QStringLiteral("cropTo")), QStringLiteral("Bounding Box"));
    QCOMPARE(seen.fields.value(QStringLiteral("antiAliased")), QStringLiteral("1"));
    QCOMPARE(seen.fields.value(QStringLiteral("widthUnit")), QStringLiteral("Inches"));
    QCOMPARE(seen.fields.value(QStringLiteral("heightUnit")), QStringLiteral("Inches"));
    QCOMPARE(seen.fields.value(QStringLiteral("resolution")), QStringLiteral("300"));
    QCOMPARE(seen.fields.value(QStringLiteral("resolutionUnit")), QStringLiteral("Pixels/Inch"));
    QCOMPARE(seen.fields.value(QStringLiteral("mode")), QStringLiteral("RGB Color"));
    QCOMPARE(seen.fields.value(QStringLiteral("depth")), QStringLiteral("8 bit"));
    QCOMPARE(seen.fields.value(QStringLiteral("thumbnailSize")), QStringLiteral("Small"));
    QCOMPARE(seen.fields.value(QStringLiteral("suppress")), QStringLiteral("0"));
    QCOMPARE(seen.fields.value(QStringLiteral("constrain")), QStringLiteral("1"));
}

// pdf_thumbnail_size
void ImportPdfTest::thumbnailSizeChangesGrid()
{
    REQUIRE_GS();
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    QStringList sizes;
    window->setImportDialogDriverForTest([&sizes](QDialog& dialog) {
        auto* pdf = qobject_cast<ImportPdfDialog*>(&dialog);
        if (!pdf) {
            return false;
        }
        sizes << pdf->fieldForTest(QStringLiteral("iconSize"));
        pdf->setFieldForTest(QStringLiteral("thumbnailSize"), QStringLiteral("Large"));
        sizes << pdf->fieldForTest(QStringLiteral("iconSize"));
        pdf->setFieldForTest(QStringLiteral("thumbnailSize"), QStringLiteral("Fit Page"));
        sizes << pdf->fieldForTest(QStringLiteral("iconSize"));
        return false;
    });
    window->openDocumentAtPath(write(dir, QStringLiteral("sample.pdf"), redPage()));
    QCOMPARE(sizes.size(), 3);
    QCOMPARE(sizes[0], QStringLiteral("64"));
    QCOMPARE(sizes[1], QStringLiteral("128"));
    QVERIFY2(sizes[2].toInt() > 128, qPrintable(sizes[2]));
}

// pdf_bounding_box
void ImportPdfTest::boundingBoxSetsSize()
{
    REQUIRE_GS();
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    Seen seen;
    window->setImportDialogDriverForTest(driver(
        window.get(), &seen, {}, {QStringLiteral("width"), QStringLiteral("height")}, {}, false));
    window->openDocumentAtPath(write(dir, QStringLiteral("sample.pdf"), redPage()));
    QVERIFY(qAbs(seen.fields.value(QStringLiteral("width")).toDouble() - 60.0 / 72.0) < 0.01);
    QVERIFY(qAbs(seen.fields.value(QStringLiteral("height")).toDouble() - 30.0 / 72.0) < 0.01);
}

// pdf_constrain_proportions
void ImportPdfTest::constrainKeepsAspect()
{
    REQUIRE_GS();
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    QStringList heights;
    window->setImportDialogDriverForTest([&heights](QDialog& dialog) {
        auto* pdf = qobject_cast<ImportPdfDialog*>(&dialog);
        if (!pdf) {
            return false;
        }
        pdf->setFieldForTest(QStringLiteral("cropTo"), QStringLiteral("Media Box"));
        pdf->setFieldForTest(QStringLiteral("width"), QStringLiteral("4"));
        heights << pdf->fieldForTest(QStringLiteral("height"));
        pdf->setFieldForTest(QStringLiteral("constrain"), QStringLiteral("0"));
        pdf->setFieldForTest(QStringLiteral("width"), QStringLiteral("3"));
        heights << pdf->fieldForTest(QStringLiteral("height"));
        return false;
    });
    window->openDocumentAtPath(write(dir, QStringLiteral("sample.pdf"), redPage()));
    QCOMPARE(heights, QStringList({QStringLiteral("2.000"), QStringLiteral("2.000")}));
}

// pdf_resolution_keeps_size
void ImportPdfTest::resolutionKeepsPhysicalSize()
{
    REQUIRE_GS();
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    QStringList seen;
    window->setImportDialogDriverForTest([&seen](QDialog& dialog) {
        auto* pdf = qobject_cast<ImportPdfDialog*>(&dialog);
        if (!pdf) {
            return false;
        }
        pdf->setFieldForTest(QStringLiteral("cropTo"), QStringLiteral("Media Box"));
        seen << pdf->fieldForTest(QStringLiteral("title"));
        pdf->setFieldForTest(QStringLiteral("resolution"), QStringLiteral("150"));
        seen << pdf->fieldForTest(QStringLiteral("width"))
             << pdf->fieldForTest(QStringLiteral("height"))
             << pdf->fieldForTest(QStringLiteral("title"));
        return false;
    });
    window->openDocumentAtPath(write(dir, QStringLiteral("sample.pdf"), redPage()));
    QCOMPARE(seen.size(), 4);
    QCOMPARE(seen[0], QStringLiteral("Image Size: %1").arg(imageSizeSummary(600.0 * 300 * 3)));
    QCOMPARE(seen[1], QStringLiteral("2.000"));
    QCOMPARE(seen[2], QStringLiteral("1.000"));
    QCOMPARE(seen[3], QStringLiteral("Image Size: %1").arg(imageSizeSummary(300.0 * 150 * 3)));
}

// pdf_multipage_max_size
void ImportPdfTest::multiPageShowsLargestBox()
{
    REQUIRE_GS();
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    Seen seen;
    window->setImportDialogDriverForTest(driver(
        window.get(), &seen,
        {{QStringLiteral("cropTo"), QStringLiteral("Media Box")},
         {QStringLiteral("select"), QStringLiteral("1,2")}},
        {QStringLiteral("width"), QStringLiteral("height")}));
    const QByteArray two = pages({{"0 0 144 72", kRedRect}, {"0 0 288 144", kRedRect}});
    QVERIFY(window->openDocumentAtPath(write(dir, QStringLiteral("two.pdf"), two)));
    QCOMPARE(seen.fields.value(QStringLiteral("width")), QStringLiteral("4.000"));
    QCOMPARE(seen.fields.value(QStringLiteral("height")), QStringLiteral("2.000"));
    QCOMPARE(window->documentCount(), 2);
    QCOMPARE(window->viewAt(0)->document_width(), 600);
    QCOMPARE(window->viewAt(0)->document_height(), 300);
    QCOMPARE(window->viewAt(1)->document_width(), 1200);
    QCOMPARE(window->viewAt(1)->document_height(), 600);
}

// pdf_render_page
void ImportPdfTest::opensPageOnTransparentLayer()
{
    REQUIRE_GS();
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    Seen seen;
    window->setImportDialogDriverForTest(driver(
        window.get(), &seen,
        {{QStringLiteral("cropTo"), QStringLiteral("Media Box")},
         {QStringLiteral("resolution"), QStringLiteral("72")}}));
    QVERIFY(window->openDocumentAtPath(write(dir, QStringLiteral("sample.pdf"), redPage())));
    PictureView* view = window->activeView();
    QCOMPARE(view->document_width(), 144);
    QCOMPARE(view->document_height(), 72);
    QCOMPARE(view->layer_count(), 1);
    QCOMPARE(view->layer_name(0), QStringLiteral("Layer 1"));
    QVERIFY(QString(image_resolution(*view)).startsWith(QStringLiteral("72")));
    QVERIFY(document_api::filePath(*view).isEmpty());
    const QColor inside = pixel(view, 50, 37);
    QCOMPARE(inside.alpha(), 255);
    QCOMPARE(inside.red(), 255);
    QCOMPARE(inside.green(), 0);
    QCOMPARE(pixel(view, 5, 5).alpha(), 0);
}

// pdf_mode_depth
void ImportPdfTest::modeAndDepthApply()
{
    REQUIRE_GS();
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    const QString path = write(dir, QStringLiteral("sample.pdf"), redPage());
    Seen seen;
    window->setImportDialogDriverForTest(driver(
        window.get(), &seen,
        {{QStringLiteral("resolution"), QStringLiteral("72")},
         {QStringLiteral("mode"), QStringLiteral("CMYK Color")},
         {QStringLiteral("depth"), QStringLiteral("16 bit")}}));
    QVERIFY(window->openDocumentAtPath(path));
    QCOMPARE(QString(image_mode(*window->activeView())), QStringLiteral("cmyk"));
    QCOMPARE(image_depth_bits(*window->activeView()), 16);

    window->setImportDialogDriverForTest(driver(
        window.get(), &seen,
        {{QStringLiteral("resolution"), QStringLiteral("72")},
         {QStringLiteral("mode"), QStringLiteral("Grayscale")}}));
    QVERIFY(window->openDocumentAtPath(path));
    QCOMPARE(QString(image_mode(*window->activeView())), QStringLiteral("grayscale"));
    QCOMPARE(image_depth_bits(*window->activeView()), 8);
}

// pdf_multi_select_documents
void ImportPdfTest::multiSelectOpensDocuments()
{
    REQUIRE_GS();
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    Seen seen;
    window->setImportDialogDriverForTest(driver(
        window.get(), &seen,
        {{QStringLiteral("resolution"), QStringLiteral("36")},
         {QStringLiteral("select"), QStringLiteral("1,3")}},
        {QStringLiteral("selectedLabel")}));
    const QByteArray deck = pages(
        {{"0 0 144 72", kRedRect}, {"0 0 144 72", kRedRect}, {"0 0 144 72", kRedRect}});
    QVERIFY(window->openDocumentAtPath(write(dir, QStringLiteral("deck.pdf"), deck)));
    QCOMPARE(seen.fields.value(QStringLiteral("selectedLabel")),
             QStringLiteral("2 of 3 page(s) selected"));
    QCOMPARE(window->documentCount(), 2);
    QCOMPARE(window->documentName(0), QStringLiteral("deck-1.pdf"));
    QCOMPARE(window->documentName(1), QStringLiteral("deck-3.pdf"));
}

// pdf_images_mode_lists
void ImportPdfTest::imagesModeListsImages()
{
    REQUIRE_GS();
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    Seen seen;
    window->setImportDialogDriverForTest(
        driver(window.get(), &seen, {{QStringLiteral("images"), QStringLiteral("1")}},
               {QStringLiteral("selectedLabel"), QStringLiteral("count"),
                QStringLiteral("imagesEnabled")},
               {}, false));
    window->openDocumentAtPath(write(dir, QStringLiteral("pictures.pdf"), imagesPdf()));
    QCOMPARE(seen.fields.value(QStringLiteral("imagesEnabled")), QStringLiteral("1"));
    QCOMPARE(seen.fields.value(QStringLiteral("count")), QStringLiteral("2"));
    QCOMPARE(seen.fields.value(QStringLiteral("selectedLabel")),
             QStringLiteral("1 of 2 image(s) selected"));
}

// pdf_images_mode_opens
void ImportPdfTest::imagesModeOpensImage()
{
    REQUIRE_GS();
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    Seen seen;
    window->setImportDialogDriverForTest(
        driver(window.get(), &seen, {{QStringLiteral("images"), QStringLiteral("1")}}));
    QVERIFY(window->openDocumentAtPath(write(dir, QStringLiteral("pictures.pdf"), imagesPdf())));
    PictureView* view = window->activeView();
    QCOMPARE(view->document_width(), 40);
    QCOMPARE(view->document_height(), 30);
    const QByteArray samples = imageSamples();
    for (const QPoint at : {QPoint(0, 0), QPoint(39, 0), QPoint(17, 21), QPoint(39, 29)}) {
        const int i = (at.y() * 40 + at.x()) * 3;
        const QColor got = pixel(view, at.x(), at.y());
        QCOMPARE(got.red(), int(uchar(samples[i])));
        QCOMPARE(got.green(), int(uchar(samples[i + 1])));
        QCOMPARE(got.blue(), int(uchar(samples[i + 2])));
    }
}

namespace {

// `plain` re-written by Ghostscript with user password `kooka`.
QString encryptedPdf(const QTemporaryDir& dir)
{
    const QString plain = write(dir, QStringLiteral("plain.pdf"), redPage());
    const QString locked = dir.filePath(QStringLiteral("locked.pdf"));
    QProcess::execute(QStringLiteral("gs"),
                      {QStringLiteral("-q"), QStringLiteral("-dSAFER"), QStringLiteral("-dBATCH"),
                       QStringLiteral("-dNOPAUSE"), QStringLiteral("-sDEVICE=pdfwrite"),
                       QStringLiteral("-sOwnerPassword=owner"),
                       QStringLiteral("-sUserPassword=kooka"), QStringLiteral("-dEncryptionR=3"),
                       QStringLiteral("-dKeyLength=128"), QStringLiteral("-o"), locked, plain});
    return locked;
}

} // namespace

// pdf_password_prompt
void ImportPdfTest::passwordPromptOpens()
{
    REQUIRE_GS();
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    const QString locked = encryptedPdf(dir);
    QVERIFY(QFile::exists(locked));
    auto window = test::makeMainWindow();
    Seen seen;
    window->setImportDialogDriverForTest(driver(window.get(), &seen,
                                                {{QStringLiteral("resolution"), QStringLiteral("72")}},
                                                {QStringLiteral("count")},
                                                {QStringLiteral("kooka")}));
    QVERIFY(window->openDocumentAtPath(locked));
    QCOMPARE(seen.prompts, 1);
    QCOMPARE(seen.fields.value(QStringLiteral("count")), QStringLiteral("1"));
    QCOMPARE(window->documentCount(), 1);
}

// pdf_wrong_password
void ImportPdfTest::wrongPasswordAsksAgain()
{
    REQUIRE_GS();
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    const QString locked = encryptedPdf(dir);
    auto window = test::makeMainWindow();
    Seen seen;
    window->setImportDialogDriverForTest(
        driver(window.get(), &seen, {}, {}, {QStringLiteral("wrong")}));
    QVERIFY(!window->openDocumentAtPath(locked));
    QCOMPARE(seen.prompts, 2);
    QCOMPARE(seen.pdfDialogs, 0);
    QCOMPARE(window->documentCount(), 0);
}

// pdf_suppress_warnings
void ImportPdfTest::suppressWarningsHidesMessage()
{
    REQUIRE_GS();
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    // Every cross-reference offset is off by five: Ghostscript repairs the
    // file and says so.
    QByteArray damaged = redPage();
    const qsizetype xref = damaged.indexOf("xref\n");
    QByteArray tail = damaged.mid(xref);
    QByteArray shifted;
    for (const QByteArray& line : tail.split('\n')) {
        if (line.endsWith(" 00000 n ")) {
            shifted += QByteArray::number(line.left(10).toLongLong() + 5).rightJustified(10, '0')
                       + " 00000 n \n";
        } else {
            shifted += line + '\n';
        }
    }
    damaged = damaged.left(xref) + shifted;
    const QString path = write(dir, QStringLiteral("damaged.pdf"), damaged);
    auto window = test::makeMainWindow();
    Seen loud;
    window->setImportDialogDriverForTest(driver(
        window.get(), &loud, {{QStringLiteral("resolution"), QStringLiteral("72")}}));
    QVERIFY(window->openDocumentAtPath(path));
    QCOMPARE(loud.warnings, 1);
    Seen quiet;
    window->setImportDialogDriverForTest(
        driver(window.get(), &quiet,
               {{QStringLiteral("resolution"), QStringLiteral("72")},
                {QStringLiteral("suppress"), QStringLiteral("1")}}));
    QVERIFY(window->openDocumentAtPath(path));
    QCOMPARE(quiet.warnings, 0);
    QCOMPARE(window->documentCount(), 2);
}

// gs_missing_reports
void ImportPdfTest::missingGhostscriptReports()
{
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    qputenv("PICTURA_GHOSTSCRIPT", "/nonexistent/pictura-gs");
    auto window = test::makeMainWindow();
    Seen seen;
    window->setImportDialogDriverForTest(driver(window.get(), &seen));
    const bool pdfOpened =
        window->openDocumentAtPath(write(dir, QStringLiteral("sample.pdf"), redPage()));
    const QString pdfStatus = status(*window);
    const bool epsOpened =
        window->openDocumentAtPath(write(dir, QStringLiteral("blue.eps"), kBlueEps));
    const QString epsStatus = status(*window);
    qunsetenv("PICTURA_GHOSTSCRIPT");
    QVERIFY(!pdfOpened);
    QVERIFY(!epsOpened);
    QVERIFY2(pdfStatus.contains(QStringLiteral("Ghostscript")), qPrintable(pdfStatus));
    QVERIFY2(epsStatus.contains(QStringLiteral("Ghostscript")), qPrintable(epsStatus));
    QCOMPARE(window->documentCount(), 0);
}

// eps_dialog_defaults
void ImportPdfTest::epsDialogDefaults()
{
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    Seen seen;
    window->setImportDialogDriverForTest(driver(
        window.get(), &seen, {},
        {QStringLiteral("width"), QStringLiteral("height"), QStringLiteral("widthUnit"),
         QStringLiteral("resolution"), QStringLiteral("resolutionUnit"), QStringLiteral("mode"),
         QStringLiteral("constrain"), QStringLiteral("antiAliased")},
        {}, false));
    QVERIFY(!window->openDocumentAtPath(write(dir, QStringLiteral("blue.eps"), kBlueEps)));
    QCOMPARE(seen.epsDialogs, 1);
    QCOMPARE(seen.fields.value(QStringLiteral("width")), QStringLiteral("2.000"));
    QCOMPARE(seen.fields.value(QStringLiteral("height")), QStringLiteral("1.000"));
    QCOMPARE(seen.fields.value(QStringLiteral("widthUnit")), QStringLiteral("Inches"));
    QCOMPARE(seen.fields.value(QStringLiteral("resolution")), QStringLiteral("300"));
    QCOMPARE(seen.fields.value(QStringLiteral("resolutionUnit")), QStringLiteral("Pixels/Inch"));
    QCOMPARE(seen.fields.value(QStringLiteral("mode")), QStringLiteral("RGB Color"));
    QCOMPARE(seen.fields.value(QStringLiteral("constrain")), QStringLiteral("1"));
    QCOMPARE(seen.fields.value(QStringLiteral("antiAliased")), QStringLiteral("1"));
}

// eps_render
void ImportPdfTest::epsOpensAtSize()
{
    REQUIRE_GS();
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    Seen seen;
    window->setImportDialogDriverForTest(
        driver(window.get(), &seen, {{QStringLiteral("resolution"), QStringLiteral("72")}}));
    QVERIFY(window->openDocumentAtPath(write(dir, QStringLiteral("blue.eps"), kBlueEps)));
    PictureView* view = window->activeView();
    QCOMPARE(view->document_width(), 144);
    QCOMPARE(view->document_height(), 72);
    QCOMPARE(window->documentName(0), QStringLiteral("blue.eps"));
    for (const QPoint at : {QPoint(0, 0), QPoint(143, 71), QPoint(72, 36)}) {
        QCOMPARE(pixel(view, at.x(), at.y()).rgba(), QColor(0, 0, 255).rgba());
    }
}

// eps_cancel
void ImportPdfTest::epsCancelOpensNothing()
{
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    Seen seen;
    window->setImportDialogDriverForTest(driver(window.get(), &seen, {}, {}, {}, false));
    QVERIFY(!window->openDocumentAtPath(write(dir, QStringLiteral("blue.eps"), kBlueEps)));
    QCOMPARE(seen.epsDialogs, 1);
    QCOMPARE(window->documentCount(), 0);
}

// open_as_command
void ImportPdfTest::openAsIsLive()
{
    test::ScopedStateHome stateHome;
    auto window = test::makeMainWindow();
    QAction* action = window->registry()->action(QString::fromLatin1(command_ids::FileOpenAs));
    QVERIFY(action);
    QVERIFY(action->isEnabled());
    QCOMPARE(action->shortcut(), QKeySequence(QStringLiteral("Ctrl+Alt+Shift+O")));
}

// open_as_forces_format
void ImportPdfTest::openAsForcesFormat()
{
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    QImage image(12, 8, QImage::Format_RGB32);
    image.fill(QColor(10, 200, 30));
    const QString path = dir.filePath(QStringLiteral("photo.dat"));
    QVERIFY(image.save(path, "PNG"));
    auto window = test::makeMainWindow();
    const int png = int(openAsFileFilters().indexOf(QStringLiteral("PNG (*.PNG)")));
    QVERIFY(png >= 0);
    QVERIFY(window->openAsPath(path, png));
    QCOMPARE(window->activeView()->document_width(), 12);
    QCOMPARE(pixel(window->activeView(), 3, 3).rgba(), QColor(10, 200, 30).rgba());
}

// open_as_mismatch
void ImportPdfTest::openAsReportsMismatch()
{
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    QImage image(4, 4, QImage::Format_RGB32);
    image.fill(Qt::red);
    const QString path = dir.filePath(QStringLiteral("photo.png"));
    QVERIFY(image.save(path));
    auto window = test::makeMainWindow();
    const int pdfFormat =
        int(openAsFileFilters().indexOf(QStringLiteral("Generic PDF (*.PDF *.PDP *.AI)")));
    QVERIFY(pdfFormat >= 0);
    QVERIFY(!window->openAsPath(path, pdfFormat));
    QCOMPARE(status(*window),
             QStringLiteral("Could not open photo.png: format mismatch or damaged file."));
    QCOMPARE(window->documentCount(), 0);
}

// A file whose signature matches the chosen Open As format but whose decode
// fails still reports the mismatch message instead of removing the tab silently.
void ImportPdfTest::openAsReportsCorruptDecode()
{
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    const QString path = write(dir, QStringLiteral("corrupt.png"),
                               QByteArray("\x89PNG\r\n\x1a\nnot a png"));
    auto window = test::makeMainWindow();
    const int pngFormat = int(openAsFileFilters().indexOf(QStringLiteral("PNG (*.PNG)")));
    QVERIFY(pngFormat >= 0);
    QVERIFY(!window->openAsPath(path, pngFormat));
    QCOMPARE(status(*window),
             QStringLiteral("Could not open corrupt.png: format mismatch or damaged file."));
    QCOMPARE(window->documentCount(), 0);
}

// open_filters_pdf_eps
void ImportPdfTest::openFiltersListPdfEps()
{
    const QStringList& filters = openFileFilters();
    QVERIFY(filters.contains(QStringLiteral("Generic PDF (*.PDF *.PDP *.AI)")));
    QVERIFY(filters.contains(QStringLiteral("Generic EPS (*.EPS *.EPSF *.PS)")));
    const QString all = filters.first();
    for (const char* pattern : {"*.PDF", "*.AI", "*.EPS", "*.EPSF", "*.PS"}) {
        QVERIFY2(all.contains(QLatin1String(pattern)), pattern);
    }
}

// Every entry point inserts the loading tab: a generic PDF's chosen page and an
// EPS's render both show it while the worker runs.
void ImportPdfTest::pdfEntryPointUsesLoadingTab()
{
    REQUIRE_GS();
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    window->setOpenDelayForTest(200);
    bool sawLoadingTab = false;
    window->setImportDialogDriverForTest([&](QDialog&) {
        QTimer::singleShot(50, window.get(), [&] {
            sawLoadingTab = window->documentCount() == 1 && window->isLoadingDocument(0);
        });
        return true;
    });
    QVERIFY(window->openDocumentAtPath(write(dir, QStringLiteral("loading.pdf"), redPage())));
    QVERIFY2(sawLoadingTab, "the chosen PDF page renders in a loading tab");
}

void ImportPdfTest::epsEntryPointUsesLoadingTab()
{
    REQUIRE_GS();
    test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = test::makeMainWindow();
    window->setOpenDelayForTest(200);
    bool sawLoadingTab = false;
    window->setImportDialogDriverForTest([&](QDialog&) {
        QTimer::singleShot(50, window.get(), [&] {
            sawLoadingTab = window->documentCount() == 1 && window->isLoadingDocument(0);
        });
        return true;
    });
    QVERIFY(window->openDocumentAtPath(write(dir, QStringLiteral("loading.eps"), kBlueEps)));
    QVERIFY2(sawLoadingTab, "the EPS render runs in a loading tab");
}

int main(int argc, char *argv[])
{
    return pictura::test::shardedMain<ImportPdfTest>(argc, argv);
}
#include "tst_import_pdf.moc"
