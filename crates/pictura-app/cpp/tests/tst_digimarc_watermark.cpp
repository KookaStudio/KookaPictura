#include <QtTest/QtTest>

#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QTabBar>

#include "commands.h"
#include "digimarc_read_dialog.h"
#include "filter_commands.h"
#include "filter_preview_dialog.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"

#include "qt_test_support.h"

namespace {

const QStringList kEmbedPath = {QStringLiteral("Filter"), QStringLiteral("Digimarc"),
                                QStringLiteral("Embed Watermark")};
const QStringList kReadPath = {QStringLiteral("Filter"), QStringLiteral("Digimarc"),
                               QStringLiteral("Read Watermark")};

pictura::FilterPreviewDialog* activeFilterDialog()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<pictura::FilterPreviewDialog*>(widget)) {
            return dialog;
        }
    }
    return nullptr;
}

void closeReadDialog()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<pictura::DigimarcReadDialog*>(widget)) {
            dialog->accept();
        }
    }
}

// The document tab bar; its tab tooltip is the unelided window label, which
// carries the detected-watermark copyright symbol.
QTabBar* documentTabBar(pictura::PicturaMainWindow* window)
{
    return window->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
}

// A distinct, structured picture with enough 8x8 cells to carry every
// durability's message, so the reader has cells to correlate against.
QImage pattern(int seed)
{
    QImage image(192, 192, QImage::Format_RGB32);
    for (int y = 0; y < 192; ++y) {
        for (int x = 0; x < 192; ++x) {
            image.setPixelColor(x, y,
                                QColor((x * 4 + seed) % 256, (y * 4 + seed * 3) % 256,
                                       (x * y + seed * 7) % 256));
        }
    }
    return image;
}

} // namespace

class DigimarcWatermarkTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void embedMenuIsEnabledAndOpens();
    void dialogExposesCs6Controls();
    void commitRecordsOneStateAndCancelRestores();
    void readMenuIsEnabled();
    void readReportsEmbeddedPayload();
    void readReportsNoWatermark();
    void detectedWatermarkMarksTheTitle();
    void undetectedWatermarkLeavesTheTitlePlain();

private:
    void openCleanDocument();

    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    int cleanSerial_ = 0;
};

void DigimarcWatermarkTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);

    const QString path = dir_.filePath(QStringLiteral("pattern.png"));
    QVERIFY(pattern(11).save(path));
    QVERIFY(window_->openDocumentAtPath(path));
    window_->registry()->refresh();
}

void DigimarcWatermarkTest::openCleanDocument()
{
    // Give each open a distinct path so it opens a new document rather than
    // reactivating the first.
    const QString path =
        dir_.filePath(QStringLiteral("clean-%1.png").arg(cleanSerial_++));
    QVERIFY(pattern(200 + cleanSerial_).save(path));
    QVERIFY(window_->openDocumentAtPath(path));
    window_->registry()->refresh();
}

void DigimarcWatermarkTest::embedMenuIsEnabledAndOpens()
{
    QAction* action = window_->registry()->action(pictura::commandIdForPath(kEmbedPath));
    QVERIFY(action != nullptr);
    QVERIFY(action->isEnabled());
    QCOMPARE(action->text(), QStringLiteral("Embed Watermark…"));

    // Dispatching opens the shared filter dialog; reject it so nothing changes.
    QTimer::singleShot(0, [] {
        if (auto* dialog = activeFilterDialog()) {
            dialog->reject();
        }
    });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kEmbedPath)));
}

void DigimarcWatermarkTest::dialogExposesCs6Controls()
{
    const pictura::FilterCommandSpec* spec = pictura::filterCommandForPath(kEmbedPath);
    QVERIFY(spec != nullptr);
    QCOMPARE(spec->kind, QStringLiteral("embed-watermark"));
    QCOMPARE(static_cast<int>(pictura::filter_param_arity(spec->kind)), 4);

    pictura::FilterPreviewDialog dialog(window_->activeView(), *spec);
    QCOMPARE(dialog.windowTitle(), QStringLiteral("Embed Watermark"));

    const QList<QComboBox*> combos = dialog.findChildren<QComboBox*>();
    QCOMPARE(combos.size(), 1);
    QCOMPARE(combos.first()->count(), 2);
    QCOMPARE(combos.first()->itemText(0), QStringLiteral("Image ID"));
    QCOMPARE(combos.first()->itemText(1), QStringLiteral("Transaction ID"));

    const QList<QRadioButton*> radios = dialog.findChildren<QRadioButton*>();
    QCOMPARE(radios.size(), 3);
    QStringList names;
    for (QRadioButton* radio : radios) {
        names.append(radio->text());
    }
    QVERIFY(names.contains(QStringLiteral("Low")));
    QVERIFY(names.contains(QStringLiteral("Medium")));
    QVERIFY(names.contains(QStringLiteral("High")));

    const QList<QDoubleSpinBox*> numbers = dialog.findChildren<QDoubleSpinBox*>();
    QCOMPARE(numbers.size(), 2);
    QCOMPARE(numbers.at(0)->minimum(), 0.0);
    QCOMPARE(numbers.at(0)->maximum(), 8388607.0);
    QCOMPARE(numbers.at(1)->minimum(), 0.0);
    QCOMPARE(numbers.at(1)->maximum(), 16777215.0);
    dialog.reject();
}

void DigimarcWatermarkTest::commitRecordsOneStateAndCancelRestores()
{
    openCleanDocument();
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const int states = view->history_count();

    // Accept the dialog, filling in a known payload.
    QTimer::singleShot(0, [] {
        auto* dialog = activeFilterDialog();
        if (!dialog) {
            return;
        }
        const QList<QDoubleSpinBox*> numbers = dialog->findChildren<QDoubleSpinBox*>();
        if (numbers.size() == 2) {
            numbers.at(0)->setValue(123456);
            numbers.at(1)->setValue(7654321);
        }
        dialog->accept();
    });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kEmbedPath)));
    QCOMPARE(view->history_count(), states + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Filter: Embed Watermark"));
    const QImage committed = view->image();

    const pictura::WatermarkRow row = pictura::read_watermark(*view);
    QVERIFY(row.found);
    QCOMPARE(row.creator_id, 123456);
    QCOMPARE(row.id, 7654321);

    // Cancelling the dialog leaves the document exactly as it was.
    QTimer::singleShot(0, [] {
        if (auto* dialog = activeFilterDialog()) {
            dialog->reject();
        }
    });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kEmbedPath)));
    QCOMPARE(view->history_count(), states + 1);
    QCOMPARE(view->image(), committed);
}

void DigimarcWatermarkTest::readMenuIsEnabled()
{
    QAction* action = window_->registry()->action(pictura::commandIdForPath(kReadPath));
    QVERIFY(action != nullptr);
    QVERIFY(action->isEnabled());
}

void DigimarcWatermarkTest::readReportsEmbeddedPayload()
{
    openCleanDocument();
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const QList<double> params = {1.0, 123456.0, 7654321.0, 2.0}; // Transaction ID, High
    QVERIFY(pictura::apply_filter_params(*view, QStringLiteral("embed-watermark"), params));

    const pictura::WatermarkRow row = pictura::read_watermark(*view);
    QVERIFY(row.found);
    QCOMPARE(row.kind, QStringLiteral("Transaction ID"));
    QCOMPARE(row.creator_id, 123456);
    QCOMPARE(row.id, 7654321);
}

void DigimarcWatermarkTest::readReportsNoWatermark()
{
    openCleanDocument();
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const pictura::WatermarkRow row = pictura::read_watermark(*view);
    QVERIFY(!row.found);
}

void DigimarcWatermarkTest::detectedWatermarkMarksTheTitle()
{
    openCleanDocument();
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const QList<double> params = {0.0, 7.0, 9.0, 2.0}; // Image ID, High
    QVERIFY(pictura::apply_filter_params(*view, QStringLiteral("embed-watermark"), params));

    QTabBar* bar = documentTabBar(window_.get());
    QVERIFY(bar != nullptr);
    const int tab = bar->count() - 1;
    QTimer::singleShot(0, [] { closeReadDialog(); });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kReadPath)));

    QVERIFY2(bar->tabToolTip(tab).contains(QChar(0x00A9)), qPrintable(bar->tabToolTip(tab)));
}

void DigimarcWatermarkTest::undetectedWatermarkLeavesTheTitlePlain()
{
    openCleanDocument();
    QTabBar* bar = documentTabBar(window_.get());
    QVERIFY(bar != nullptr);
    const int tab = bar->count() - 1;

    QTimer::singleShot(0, [] { closeReadDialog(); });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kReadPath)));

    QVERIFY2(!bar->tabToolTip(tab).contains(QChar(0x00A9)), qPrintable(bar->tabToolTip(tab)));
}

QTEST_MAIN(DigimarcWatermarkTest)

#include "tst_digimarc_watermark.moc"
