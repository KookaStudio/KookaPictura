// File > Print and Print One Copy (#66): the dialog previews the page, keeps
// its settings on Done / Print, and Save as PDF writes the job.

#include <QtTest/QtTest>

#include "commands.h"
#include "frame.h"
#include "print_dialog.h"

#include <QtCore/QTemporaryDir>
#include <QtPrintSupport/QPrinter>
#include <QtGui/QAction>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
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

} // namespace

class PrintTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void previewAndOrientation();
    void printSettingsPersistPageSetup();
    void saveAsPdf();
    void menuCommands();

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

QTEST_MAIN(PrintTest)
#include "tst_print.moc"
