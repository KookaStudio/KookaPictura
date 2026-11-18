// File > Print and Print One Copy (#66): the dialog previews the page, keeps
// its settings on Done / Print, and Save as PDF writes the job.

#include <QtTest/QtTest>

#include "commands.h"
#include "dialogs.h"
#include "frame.h"
#include "print_dialog.h"
#include "print_job.h"
#include "api/proofing_bridge.h"
#include "proof_setup_dialog.h"

#include <QtCore/QFile>
#include <QtCore/QDir>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QRegularExpression>
#include <QtCore/QScopeGuard>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtPrintSupport/QPrinter>
#include <QtGui/QAction>
#include <QtGui/QPainter>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>

#include "qt_test_support.h"

namespace {

template <typename T>
T* child(QWidget& parent, const char* name)
{
    return parent.findChild<T*>(QString::fromLatin1(name));
}

// The bounding box of the preview's page: the near-white pixels on the grey.
QRect pageBounds(const QImage& preview)
{
    QRect bounds;
    for (int y = 0; y < preview.height(); ++y) {
        for (int x = 0; x < preview.width(); ++x) {
            if (qGray(preview.pixel(x, y)) > 230) {
                bounds = bounds.united(QRect(x, y, 1, 1));
            }
        }
    }
    return bounds;
}

QImage picture()
{
    QImage image(120, 80, QImage::Format_RGBA8888);
    image.fill(QColor(200, 30, 30));
    return image;
}

QJsonObject managedOptions(int device)
{
    const auto profile = pictura::describePrintProfile(QStringLiteral("builtin:%1").arg(device));
    return {{QStringLiteral("managed"), true}, {QStringLiteral("profile"), profile.name},
            {QStringLiteral("profilePath"), profile.path}, {QStringLiteral("components"), profile.components}};
}

QByteArray printPdf(pictura::PrintDialog& dialog, const QString& path)
{
    dialog.setPdfPathProvider([path]() { return path; });
    child<QPushButton>(dialog, "printPrint")->click();
    QFile file(path);
    return dialog.result() == QDialog::Accepted && file.open(QIODevice::ReadOnly) ? file.readAll()
                                                                                  : QByteArray();
}

QJsonObject documentPrintOptions(const pictura::PictureView& view)
{
    return QJsonDocument::fromJson(pictura::print_document_settings(view).toUtf8()).object();
}

QString printOptionsText(const QJsonObject& values)
{
    return QString::fromUtf8(QJsonDocument(values).toJson(QJsonDocument::Compact));
}

} // namespace

class PrintTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void previewAndOrientation();
    void printSettingsPersistPageSetup();
    void saveAsPdf();
    void menuCommands();
    void applicationManagedPdf();
    void invalidManagedProfileBlocksOutput();
    void previewAidsLeaveOutputSamplesAlone();
    void hardProofUsesLocalSetup();
    void hardProofModeAndSavedSetup();
    void outputMarksMatchPdfLayout();
    void outputMarksRespectPrintableBounds();
    void printOneCopyUsesResolvedSettings();
    void acceptedDefaultsSurviveRestartAndReopen();
    void managedCupsEligibility();
    void pageSetupReachesOutput();
    void inheritedManagedSettingsPersist();
    void proofProfilePathTakesPrecedence();
    void pendingPrintMetadataRequiresNativeSave();

private:
    pictura::test::ScopedStateHome stateHome_;
};

void PrintTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
}

void PrintTest::previewAndOrientation()
{
    pictura::lastPrintSettings() = {QString(), 1, false};
    pictura::PrintDialog dialog(picture());
    auto* printer = child<QComboBox>(dialog, "printPrinter");
    auto* landscape = child<QPushButton>(dialog, "printLandscape");
    auto* paperWhite = child<QCheckBox>(dialog, "printPaperWhite");
    auto* size = child<QLabel>(dialog, "printPageSize");
    QVERIFY(printer && landscape && paperWhite && size);

    // Save as PDF is always offered, last, and is what an empty default picks.
    QCOMPARE(printer->itemText(printer->count() - 1), QStringLiteral("Save as PDF"));
    QVERIFY(printer->currentData().toString().isEmpty());
    QCOMPARE(size->text(), QStringLiteral("8.5 in x 11 in"));

    // Portrait is taller than wide, landscape the reverse, the picture centred.
    QRect page = pageBounds(dialog.pagePreviewForTest());
    QVERIFY2(page.height() > page.width(), "portrait page");
    const QImage portrait = dialog.pagePreviewForTest();
    QCOMPARE(QColor(portrait.pixel(page.center())).red(), 200);
    landscape->click();
    QCOMPARE(size->text(), QStringLiteral("11 in x 8.5 in"));
    page = pageBounds(dialog.pagePreviewForTest());
    QVERIFY2(page.width() > page.height(), "landscape page");

    // Show Paper White tints the paper.
    const QPoint corner = page.topLeft() + QPoint(3, 3);
    const QRgb white = dialog.pagePreviewForTest().pixel(corner);
    paperWhite->setChecked(true);
    QVERIFY(dialog.pagePreviewForTest().pixel(corner) != white);

    // Cancel keeps the old defaults.
    dialog.reject();
    QVERIFY(!pictura::lastPrintSettings().landscape);
}

void PrintTest::printSettingsPersistPageSetup()
{
    pictura::lastPrintSettings() = {};
    pictura::PrintDialog dialog(picture());
    auto* setup = child<QPushButton>(dialog, "printPageSetup");
    auto* size = child<QLabel>(dialog, "printPageSize");
    QVERIFY(setup && size);
    QVERIFY2(setup->isEnabled(), "page setup is available for Save as PDF");
    dialog.setPageSetupProvider([](QPrinter& printer) {
        printer.setPageSize(QPageSize(QPageSize::A4));
        printer.setPaperSource(QPrinter::Cassette);
        return true;
    });
    setup->click();

    QCOMPARE(size->text(), QStringLiteral("8.27 in x 11.7 in"));
    const pictura::PrintSettings chosen = dialog.settings();
    QCOMPARE(int(chosen.pageSize.id()), int(QPageSize::A4));
    QCOMPARE(int(chosen.paperSource), int(QPrinter::Cassette));

    QPrinter printer;
    pictura::configurePrinter(printer, chosen, QString());
    QCOMPARE(int(printer.pageLayout().pageSize().id()), int(QPageSize::A4));

    child<QPushButton>(dialog, "printDone")->click();
    QCOMPARE(int(pictura::lastPrintSettings().pageSize.id()), int(QPageSize::A4));
    QCOMPARE(int(pictura::lastPrintSettings().paperSource), int(QPrinter::Cassette));
    QCOMPARE(int(pictura::readPrintSettings().pageSize.id()), int(QPageSize::A4));
}

void PrintTest::saveAsPdf()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("job.pdf"));
    pictura::lastPrintSettings() = {QString(), 1, false};
    pictura::PrintDialog dialog(picture());
    dialog.setPdfPathProvider([path]() { return path; });
    child<QSpinBox>(dialog, "printCopies")->setValue(2);
    child<QPushButton>(dialog, "printLandscape")->click();
    child<QPushButton>(dialog, "printPrint")->click();
    QCOMPARE(dialog.result(), int(QDialog::Accepted));

    QFile pdf(path);
    QVERIFY(pdf.open(QIODevice::ReadOnly));
    QVERIFY(pdf.read(5) == "%PDF-");
    QVERIFY(pdf.size() > 1000);
    // Print keeps the settings for next time and for Print One Copy.
    QCOMPARE(pictura::lastPrintSettings().copies, 2);
    QVERIFY(pictura::lastPrintSettings().landscape);

    // A cancelled file choice prints nothing and leaves the dialog open.
    pictura::PrintDialog again(picture());
    again.setPdfPathProvider([]() { return QString(); });
    again.show();
    child<QPushButton>(again, "printPrint")->click();
    QVERIFY(again.isVisible());
    again.reject();

    // printImage refuses a null image.
    QPrinter printer;
    pictura::configurePrinter(printer, {}, dir.filePath(QStringLiteral("none.pdf")));
    QVERIFY(!pictura::printImage(printer, QImage()));
}

void PrintTest::menuCommands()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    pictura::CommandRegistry* registry = window->registry();
    QAction* print = registry->action(
        pictura::commandIdForPath({QStringLiteral("File"), QStringLiteral("Print…")}));
    QAction* once = registry->action(
        pictura::commandIdForPath({QStringLiteral("File"), QStringLiteral("Print One Copy")}));
    QVERIFY(print && once);
    QCOMPARE(print->shortcut(), QKeySequence(QStringLiteral("Ctrl+P")));
    registry->refresh();
    QVERIFY(!print->isEnabled());
    QVERIFY(window->newDocument(QStringLiteral("Print"), 16, 16, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    registry->refresh();
    QVERIFY(print->isEnabled() && once->isEnabled());
    const QImage image = pictura::printableImage(window->activeView());
    QCOMPARE(image.size(), QSize(16, 16));
    QCOMPARE(QColor(image.pixel(8, 8)), QColor(Qt::white));
}

void PrintTest::applicationManagedPdf()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    for (const int device : {0, 1, 3}) {
        auto& saved = pictura::lastPrintSettings();
        saved = {};
        saved.options = managedOptions(device);
        pictura::PrintDialog dialog(picture());
        const QByteArray pdf = printPdf(dialog, dir.filePath(QStringLiteral("%1.pdf").arg(device)));
        QVERIFY2(pdf.contains("/ColorSpace [/ICCBased "), qPrintable(child<QLabel>(dialog, "printStatus")->text()));
        const int components = saved.options.value(QStringLiteral("components")).toInt();
        QVERIFY(pdf.contains(QByteArray("/N ") + QByteArray::number(components)));
        QCOMPARE(components, device == 0 ? 4 : device == 1 ? 1 : 3);
    }
    pictura::lastPrintSettings() = {};
    pictura::PrintDialog unmanaged(picture());
    const auto pdf = printPdf(unmanaged, dir.filePath(QStringLiteral("unmanaged.pdf")));
    QVERIFY(pdf.contains("/ColorSpace /DeviceRGB"));
}

void PrintTest::invalidManagedProfileBlocksOutput()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString broken = dir.filePath(QStringLiteral("bad.icc"));
    QFile file(broken);
    QVERIFY(file.open(QIODevice::WriteOnly));
    file.write("incorrect ICC");
    file.close();
    for (const QString& path : {broken, dir.filePath(QStringLiteral("missing.icc"))}) {
        auto& saved = pictura::lastPrintSettings();
        saved = {};
        saved.options = {{QStringLiteral("managed"), true}, {QStringLiteral("profile"), QStringLiteral("Broken")},
                         {QStringLiteral("profilePath"), path}};
        pictura::PrintDialog dialog(picture());
        const QString output = dir.filePath(QStringLiteral("bad.pdf"));
        QVERIFY(printPdf(dialog, output).isEmpty());
        QVERIFY(!QFile::exists(output));
        QVERIFY(!child<QLabel>(dialog, "printStatus")->text().isEmpty());
        QCOMPARE(dialog.settings().options.value(QStringLiteral("profilePath")).toString(), path);
    }
    const bool hadDataHome = qEnvironmentVariableIsSet("XDG_DATA_HOME");
    const QByteArray originalDataHome = qgetenv("XDG_DATA_HOME");
    const auto restore = qScopeGuard([=]() {
        if (hadDataHome) {
            qputenv("XDG_DATA_HOME", originalDataHome);
        } else {
            qunsetenv("XDG_DATA_HOME");
        }
    });
    qputenv("XDG_DATA_HOME", dir.path().toUtf8());
    QVERIFY(QDir(dir.path()).mkpath(QStringLiteral("color/icc")));
    const QByteArray profile = pictura::profileBytes(QStringLiteral("builtin:3"));
    QVERIFY(profile.size() >= 128);
    const QString one = dir.filePath(QStringLiteral("color/icc/one.icc"));
    const QString two = dir.filePath(QStringLiteral("color/icc/two.icc"));
    // Distinct creator signatures keep both files after ICC byte deduplication.
    for (const auto& entry : {qMakePair(one, "kp01"), qMakePair(two, "kp02")}) {
        QByteArray contents = profile;
        contents.replace(80, 4, entry.second);
        QFile output(entry.first);
        QVERIFY(output.open(QIODevice::WriteOnly));
        QCOMPARE(output.write(contents), contents.size());
    }
    const auto imported = pictura::describePrintProfile(one);
    const auto other = pictura::describePrintProfile(two);
    QCOMPARE(imported.components, 3);
    QCOMPARE(other.components, 3);
    QVERIFY(!imported.name.isEmpty());
    QCOMPARE(imported.name, other.name);
    auto& saved = pictura::lastPrintSettings();
    saved = {};
    saved.options = {{QStringLiteral("managed"), true}, {QStringLiteral("profile"), imported.name}};
    pictura::PrintDialog ambiguous(picture());
    QVERIFY(printPdf(ambiguous, dir.filePath(QStringLiteral("ambiguous.pdf"))).isEmpty());
    QVERIFY(!child<QLabel>(ambiguous, "printStatus")->text().isEmpty());
}

void PrintTest::previewAidsLeaveOutputSamplesAlone()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    auto& saved = pictura::lastPrintSettings();
    saved = {};
    saved.options = managedOptions(0);
    QImage patches(80, 40, QImage::Format_RGBA8888);
    patches.fill(QColor(45, 45, 45));
    for (int y = 0; y < patches.height(); ++y) {
        for (int x = patches.width() / 2; x < patches.width(); ++x) {
            patches.setPixelColor(x, y, QColor(0, 0, 255));
        }
    }
    pictura::PrintDialog before(patches);
    const QByteArray original = printPdf(before, dir.filePath(QStringLiteral("original.pdf")));
    QVERIFY(!original.isEmpty());
    pictura::PrintDialog after(patches);
    const QImage initial = after.pagePreviewForTest();
    child<QCheckBox>(after, "printMatchColors")->setChecked(true);
    QVERIFY(after.pagePreviewForTest() != initial);
    const QImage matched = after.pagePreviewForTest();
    auto* gamut = child<QCheckBox>(after, "printGamutWarning");
    QVERIFY(gamut->isEnabled());
    gamut->setChecked(true);
    QVERIFY(after.pagePreviewForTest() != matched);
    QCOMPARE(after.pagePreviewForTest().pixelColor(175, 180), QColor(230, 0, 230));
    QVERIFY(after.pagePreviewForTest().pixelColor(105, 180) != QColor(230, 0, 230));
    child<QCheckBox>(after, "printPaperWhite")->setChecked(true);
    QCOMPARE(printPdf(after, dir.filePath(QStringLiteral("preview.pdf"))), original);
}

void PrintTest::hardProofUsesLocalSetup()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Proof"), 16, 16, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    auto& view = *window->activeView();
    pictura::set_proof_setup(view, 0, 1, true, false, false);
    pictura::set_proof_colors(view, false);
    const int history = view.history_count();
    auto& saved = pictura::lastPrintSettings();
    saved = {};
    saved.options = managedOptions(3);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    pictura::PrintDialog dialog(view);
    child<QCheckBox>(dialog, "printHardProof")->setChecked(true);
    QTimer::singleShot(0, []() {
        auto* proof = qobject_cast<pictura::ProofSetupDialog*>(QApplication::activeModalWidget());
        if (proof) {
            proof->setDeviceForTest(1);
            proof->setSimulateBlackForTest(true);
            proof->accept();
        }
    });
    child<QPushButton>(dialog, "printProofSetup")->click();
    QCOMPARE(dialog.settings().options.value(QStringLiteral("proof")).toObject().value(QStringLiteral("name")).toString(),
             QStringLiteral("Working Gray"));
    const auto output = printPdf(dialog, dir.filePath(QStringLiteral("proof.pdf")));
    QVERIFY(output.contains("/ICCBased"));
    QCOMPARE(pictura::proof_device(view), 0);
    QVERIFY(!pictura::proof_colors_active(view));
    QCOMPARE(view.history_count(), history);
}

void PrintTest::hardProofModeAndSavedSetup()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Proof settings"), 16, 16, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    auto& view = *window->activeView();
    pictura::set_proof_setup(view, 1, 1, true, false, false);
    pictura::lastPrintSettings() = {};
    pictura::PrintDialog defaults(view);
    QVERIFY(!child<QCheckBox>(defaults, "printHardProof")->isEnabled());
    QCOMPARE(defaults.settings().options.value(QStringLiteral("proof")).toObject().value(QStringLiteral("name")).toString(),
             QStringLiteral("Working Gray"));
    auto options = managedOptions(3);
    options.insert(QStringLiteral("hardProof"), true);
    const QJsonObject proof{{QStringLiteral("kind"), QStringLiteral("builtin")},
                            {QStringLiteral("name"), QStringLiteral("proofCMYK")}};
    options.insert(QStringLiteral("proof"), proof);
    QVERIFY(pictura::print_document_settings_update(view, QStringLiteral("{}"), printOptionsText(options), true).isEmpty());
    pictura::PrintDialog restored(view);
    QCOMPARE(restored.settings().options.value(QStringLiteral("proof")).toObject().value(QStringLiteral("name")).toString(),
             QStringLiteral("proofCMYK"));
    auto* hard = child<QCheckBox>(restored, "printHardProof");
    QVERIFY(hard->isEnabled() && hard->isChecked());
    auto* handling = child<QComboBox>(restored, "printColorHandling");
    handling->setCurrentIndex(0);
    QVERIFY(!hard->isEnabled());
    handling->setCurrentIndex(1);
    QVERIFY(hard->isEnabled() && hard->isChecked());
    child<QComboBox>(restored, "printProfile")->setCurrentIndex(0);
    QVERIFY(!hard->isEnabled());
    QVERIFY(!pictura::printJobPreflight(restored.settings()).isEmpty());
}

void PrintTest::outputMarksMatchPdfLayout()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    auto& saved = pictura::lastPrintSettings();
    saved = {};
    saved.options = {{QStringLiteral("cornerCrop"), true}, {QStringLiteral("centerCrop"), true},
                     {QStringLiteral("registration"), true}, {QStringLiteral("calibration"), true},
                     {QStringLiteral("filename"), true}, {QStringLiteral("description"), true},
                     {QStringLiteral("bleed"), 5}, {QStringLiteral("border"), 2},
                     {QStringLiteral("background"), QJsonArray{230, 225, 215}}};
    const auto layout = pictura::layoutPrintPage(pictura::physicalPrintPage(saved), picture().size(), saved.options,
                                               QStringLiteral("Example.psd"), QStringLiteral("Test description"));
    auto zero = saved.options;
    zero.insert(QStringLiteral("bleed"), 0);
    const auto unbled = pictura::layoutPrintPage(pictura::physicalPrintPage(saved), picture().size(), zero,
                                               QStringLiteral("Example.psd"), QStringLiteral("Test description"));
    QVERIFY(layout.marks.size() >= 23);
    QVERIFY(layout.marks.at(1).path.boundingRect() != unbled.marks.at(1).path.boundingRect());
    const QString filename = dir.filePath(QStringLiteral("marks.pdf"));
    QVERIFY2(pictura::submitPrintJob(saved, nullptr, picture(), filename, QStringLiteral("Example.psd"),
                                    QStringLiteral("Test description")).isEmpty(), "Writing marked PDF");
    QFile file(filename);
    QVERIFY(file.open(QIODevice::ReadOnly));
    const auto output = file.readAll();
    for (const auto& mark : layout.marks) {
        QVERIFY(output.contains(pictura::printPathOperators(mark.path, layout.paper.height()).toUtf8()));
    }
    QImage preview(612, 792, QImage::Format_RGB32);
    QPainter painter(&preview);
    pictura::paintPrintPage(painter, layout, picture(), Qt::white);
    painter.end();
    QCOMPARE(preview.pixelColor(1, 1), layout.printable.contains(QPointF(1, 1)) ? layout.background
                                                                             : QColor(Qt::white));
}

void PrintTest::outputMarksRespectPrintableBounds()
{
    pictura::PrintSettings settings;
    settings.pageSize = QPageSize(QSizeF(35, 35), QPageSize::Point, {}, QPageSize::ExactMatch);
    settings.options = {{QStringLiteral("cornerCrop"), true}, {QStringLiteral("centerCrop"), true},
                        {QStringLiteral("border"), 24}};
    const auto page = pictura::physicalPrintPage(settings);
    const auto layout = pictura::layoutPrintPage(page, QSizeF(20, 20), settings.options, {}, {});
    QVERIFY(layout.printable.contains(layout.image));
    QVERIFY(layout.marks.isEmpty());
    QImage square(20, 20, QImage::Format_RGBA8888);
    square.fill(Qt::red);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    pictura::lastPrintSettings() = settings;
    pictura::PrintDialog dialog(square);
    const auto output = printPdf(dialog, dir.filePath(QStringLiteral("constrained.pdf")));
    QVERIFY(output.contains("/Im0 Do Q"));
    QVERIFY(!output.contains("/Print cs"));
}

void PrintTest::printOneCopyUsesResolvedSettings()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("One copy"), 16, 16, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    auto& view = *window->activeView();
    auto& saved = pictura::lastPrintSettings();
    saved = {QStringLiteral("fixture-queue"), 3, true};
    auto document = managedOptions(3);
    document.insert(QStringLiteral("border"), 7);
    document.insert(QStringLiteral("hardProof"), true);
    QVERIFY(pictura::print_document_settings_update(view, QStringLiteral("{}"), printOptionsText(document), true).isEmpty());
    const auto settings = pictura::resolvedPrintSettings(&view);
    int submitted = 0;
    QStringList opened;
    const auto capture = [&](const pictura::PrintSettings& submission) {
        ++submitted;
        QCOMPARE(submission.copies, 1);
        QVERIFY(submission.options.value(QStringLiteral("managed")).toBool());
        QVERIFY(submission.options.value(QStringLiteral("hardProof")).toBool());
        QCOMPARE(submission.options.value(QStringLiteral("border")).toInt(), 7);
    };
    const auto accept = [](const pictura::PrintSettings&) { return QString(); };
    const auto dialog = [&](const QString& reason) { opened.append(reason); };
    pictura::routePrintOneCopy(settings, true, accept, capture, dialog);
    QCOMPARE(submitted, 1);
    QCOMPARE(saved.copies, 3);
    pictura::routePrintOneCopy(settings, false, accept, capture, dialog);
    auto pdf = settings;
    pdf.printer.clear();
    pictura::routePrintOneCopy(pdf, true, accept, capture, dialog);
    pictura::routePrintOneCopy(settings, true, [](const pictura::PrintSettings&) {
        return QStringLiteral("Unvalidated route");
    }, capture, dialog);
    QCOMPARE(submitted, 1);
    QCOMPARE(opened, QStringList({{}, {}, QStringLiteral("Unvalidated route")}));
}

void PrintTest::acceptedDefaultsSurviveRestartAndReopen()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Persist"), 16, 16, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    auto& view = *window->activeView();
    pictura::lastPrintSettings() = {};
    const int history = view.history_count();
    pictura::PrintDialog done(view);
    child<QCheckBox>(done, "print_cornerCrop")->setChecked(true);
    child<QSpinBox>(done, "printCopies")->setValue(4);
    child<QPushButton>(done, "printDone")->click();
    QCOMPARE(done.result(), int(QDialog::Accepted));
    QCOMPARE(view.history_count(), history);
    QCOMPARE(pictura::readPrintSettings().copies, 4);
    const QString path = dir.filePath(QStringLiteral("document.psd"));
    QVERIFY(view.save(path));
    auto next = pictura::readPrintSettings();
    next.options.insert(QStringLiteral("cornerCrop"), false);
    pictura::lastPrintSettings() = next;
    QVERIFY(view.open(path));
    pictura::PrintDialog after(view);
    QVERIFY(child<QCheckBox>(after, "print_cornerCrop")->isChecked());
    child<QCheckBox>(after, "print_cornerCrop")->setChecked(false);
    after.reject();
    QVERIFY(documentPrintOptions(view).value(QStringLiteral("cornerCrop")).toBool());
    pictura::PrintDialog failure(view);
    child<QCheckBox>(failure, "print_cornerCrop")->setChecked(false);
    QVERIFY(printPdf(failure, dir.filePath(QStringLiteral("missing/output.pdf"))).isEmpty());
    QVERIFY(documentPrintOptions(view).value(QStringLiteral("cornerCrop")).toBool());
    QCOMPARE(pictura::readPrintSettings().copies, 4);
}

void PrintTest::managedCupsEligibility()
{
    pictura::PrintSettings settings{QStringLiteral("fixture-queue"), 2, true,
                                   QPageSize(QPageSize::A4), QPrinter::Cassette};
    const auto profile = pictura::describePrintProfile(QStringLiteral("builtin:3"));
    const auto command = [](const QString& name, const QStringList&) { return name.toUtf8(); };
    auto route = pictura::managedPrintEvidence(settings, profile);
    route.insert(QStringLiteral("device"), QStringLiteral("Fixture printer"));
    route.insert(QStringLiteral("filters"), QStringLiteral("Fixture pass-through"));
    route.insert(QStringLiteral("fingerprint"), pictura::printQueueFingerprint(settings.printer, command));
    route.insert(QStringLiteral("evidence"), QStringLiteral("Fixture channel verification"));
    route.insert(QStringLiteral("orientation"), QStringLiteral("pdf"));
    route.insert(QStringLiteral("options"), QJsonObject{{QStringLiteral("media"), QStringLiteral("A4")},
        {QStringLiteral("media-source"), QStringLiteral("tray-2")}, {QStringLiteral("ColorManagement"), QStringLiteral("Off")}});
    const auto job = pictura::prepareManagedCups(settings, profile, route, command);
    QVERIFY(job.error.isEmpty());
    QVERIFY(job.arguments.contains(QStringLiteral("media=A4")));
    QVERIFY(job.arguments.contains(QStringLiteral("media-source=tray-2")));
    QVERIFY(!job.arguments.contains(QStringLiteral("orientation-requested=4")));
    QVERIFY(!job.arguments.contains(QStringLiteral("orientation-requested=3")));
    auto neutralRoute = route;
    neutralRoute.insert(QStringLiteral("orientation"), QStringLiteral("pdf-with-portrait-option"));
    const auto neutral = pictura::prepareManagedCups(settings, profile, neutralRoute, command);
    QVERIFY(neutral.error.isEmpty());
    QVERIFY(neutral.arguments.contains(QStringLiteral("orientation-requested=3")));
    QCOMPARE(job.arguments.value(job.arguments.indexOf(QStringLiteral("-n")) + 1), QStringLiteral("2"));
    QVERIFY(!pictura::prepareManagedCups(settings, profile, {}, command).error.isEmpty());
    QVERIFY(!pictura::prepareManagedCups(settings, profile, route, [](const QString&, const QStringList&) {
        return QByteArray("changed queue snapshot");
    }).error.isEmpty());
    auto changed = settings;
    changed.paperSource = QPrinter::Manual;
    QVERIFY(!pictura::prepareManagedCups(changed, profile, route, command).error.isEmpty());
    QVERIFY(!pictura::prepareManagedCups(settings, pictura::describePrintProfile(QStringLiteral("builtin:0")),
                                       route, command).error.isEmpty());
    const auto submit = [](const QString& name, const QStringList& arguments) -> QByteArray {
        QFile file(arguments.last());
        return name == QStringLiteral("lp") && file.open(QIODevice::ReadOnly) && file.readAll() == "%PDF-test"
            ? QByteArray("request id is fixture-queue-14 (1 file)") : QByteArray();
    };
    QVERIFY(pictura::submitManagedCups(job, "%PDF-test", submit).isEmpty());
    QVERIFY(!pictura::submitManagedCups(job, "%PDF-test", [](const QString&, const QStringList&) {
        return QByteArray();
    }).isEmpty());
}

void PrintTest::proofProfilePathTakesPrecedence()
{
    QTemporaryDir dir;
    const auto builtin = pictura::describePrintProfile(QStringLiteral("builtin:3"));
    QByteArray custom = builtin.bytes;
    custom.append('\0');
    const QString path = dir.filePath(QStringLiteral("custom.icc"));
    QFile file(path);
    QVERIFY(file.open(QIODevice::WriteOnly));
    QCOMPARE(file.write(custom), qint64(custom.size()));
    file.close();
    QJsonObject proof{{QStringLiteral("kind"), QStringLiteral("named")},
                      {QStringLiteral("name"), builtin.name}, {QStringLiteral("path"), path}};
    QCOMPARE(pictura::resolvePrintProof(proof), custom);
    QVERIFY(file.remove());
    QVERIFY(pictura::resolvePrintProof(proof).isEmpty());
    auto options = managedOptions(0);
    options.insert(QStringLiteral("hardProof"), true);
    options.insert(QStringLiteral("match"), true);
    options.insert(QStringLiteral("proof"), QJsonObject{{QStringLiteral("kind"), QStringLiteral("builtin")},
        {QStringLiteral("name"), QStringLiteral("proofCMYK")}, {QStringLiteral("path"), QStringLiteral("builtin:0")}});
    QImage blue(1, 1, QImage::Format_RGBA8888);
    blue.fill(QColor(0, 0, 255));
    pictura::PrintSettings settings;
    settings.options = options;
    QString error;
    const QImage plain = pictura::proofPrintPreview(blue, settings, error);
    QVERIFY(error.isEmpty());
    settings.options.insert(QStringLiteral("gamut"), true);
    const QImage warning = pictura::proofPrintPreview(blue, settings, error);
    QVERIFY(error.isEmpty());
    QCOMPARE(warning.pixelColor(0, 0), QColor(230, 0, 230));
    QVERIFY(warning != plain);
}

void PrintTest::inheritedManagedSettingsPersist()
{
    QTemporaryDir dir;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Inherited"), 16, 16, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    auto& view = *window->activeView();
    auto options = managedOptions(3);
    options.insert(QStringLiteral("intent"), 1);
    options.insert(QStringLiteral("bpc"), true);
    options.insert(QStringLiteral("hardProof"), true);
    options.insert(QStringLiteral("proof"), pictura::viewPrintProof(&view));
    auto changed = options;
    changed.insert(QStringLiteral("intent"), 0);
    QVERIFY(pictura::print_document_settings_update(view, printOptionsText(options),
                                                    printOptionsText(changed), true).isEmpty());
    for (const QString& suffix : {QStringLiteral("psd"), QStringLiteral("psb")}) {
        const QString path = dir.filePath(QStringLiteral("inherited.") + suffix);
        QVERIFY(view.save(path));
        QVERIFY(view.open(path));
        const auto restored = documentPrintOptions(view);
        QCOMPARE(restored.value(QStringLiteral("profile")), options.value(QStringLiteral("profile")));
        QCOMPARE(restored.value(QStringLiteral("intent")).toInt(), 0);
        QVERIFY(restored.value(QStringLiteral("hardProof")).toBool());
        QVERIFY(!restored.value(QStringLiteral("proof")).toObject().isEmpty());
    }
}

void PrintTest::pendingPrintMetadataRequiresNativeSave()
{
    QTemporaryDir dir;
    auto window = pictura::test::makeMainWindow();
    const QString png = dir.filePath(QStringLiteral("source.png"));
    QVERIFY(picture().save(png));
    QVERIFY(window->openDocumentAtPath(png));
    auto& view = *window->activeView();
    QJsonObject changed{{QStringLiteral("border"), 4}};
    QVERIFY(pictura::print_document_settings_update(view, QStringLiteral("{}"),
                                                    printOptionsText(changed), true).isEmpty());
    QVERIFY(pictura::print_metadata_pending(view));
    const QString psd = dir.filePath(QStringLiteral("saved.psd"));
    QStringList seen;
    pictura::setSaveFileNameProviderForTest([&](QWidget*, const QString&, const QString&,
                                                const QStringList& filters, QString*) {
        seen = filters;
        return psd;
    });
    const auto reset = qScopeGuard([] { pictura::setSaveFileNameProviderForTest(nullptr); });
    QVERIFY(window->saveActive());
    QCOMPARE(seen.size(), 2);
    QVERIFY(QFile::exists(psd));
    QVERIFY(!pictura::print_metadata_pending(view));
}

void PrintTest::pageSetupReachesOutput()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    auto& saved = pictura::lastPrintSettings();
    saved = {};
    saved.options = managedOptions(3);
    pictura::PrintDialog dialog(picture());
    dialog.setPageSetupProvider([](QPrinter& printer) {
        printer.setPageSize(QPageSize(QPageSize::A4));
        printer.setPaperSource(QPrinter::Cassette);
        return true;
    });
    child<QPushButton>(dialog, "printPageSetup")->click();
    QCOMPARE(child<QLabel>(dialog, "printPageSize")->text(), QStringLiteral("8.27 in x 11.7 in"));
    const QByteArray pdf = printPdf(dialog, dir.filePath(QStringLiteral("a4.pdf")));
    const QRegularExpression box(QStringLiteral(R"(/MediaBox \[0 0 ([\d.]+) ([\d.]+)\])"));
    const auto match = box.match(QString::fromLatin1(pdf));
    QVERIFY(match.hasMatch());
    QVERIFY(qAbs(match.captured(1).toDouble() - 595.28) < 1);
    QVERIFY(qAbs(match.captured(2).toDouble() - 841.89) < 1);
    QCOMPARE(saved.pageSize.id(), QPageSize::A4);
    QCOMPARE(saved.paperSource, QPrinter::Cassette);
}

int main(int argc, char* argv[])
{
    return pictura::test::shardedMain<PrintTest>(argc, argv);
}
#include "tst_print.moc"
