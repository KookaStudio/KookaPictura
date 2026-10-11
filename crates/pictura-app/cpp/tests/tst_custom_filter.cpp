#include <QtTest/QtTest>

#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>

#include "commands.h"
#include "filter_commands.h"
#include "filter_preview_dialog.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"

#include "qt_test_support.h"

namespace {

const QStringList kCustomPath = {QStringLiteral("Filter"), QStringLiteral("Other"),
                                 QStringLiteral("Custom")};

QList<double> identity()
{
    QList<double> values(pictura::kMatrixCells, 0.0);
    values[pictura::kMatrixCells / 2] = 1.0;
    return values << 1.0 << 0.0;
}

pictura::FilterPreviewDialog* activeFilterDialog()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<pictura::FilterPreviewDialog*>(widget)) {
            return dialog;
        }
    }
    return nullptr;
}

} // namespace

class CustomFilterTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void dialogFollowsCs6();
    void menuOpensPreviewsAndCommits();
    void lastFilterReusesTheKernel();

private:
    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void CustomFilterTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);

    QImage pattern(32, 32, QImage::Format_RGB32);
    for (int y = 0; y < 32; ++y) {
        for (int x = 0; x < 32; ++x) {
            pattern.setPixelColor(x, y, QColor((x * 8) % 256, (y * 8) % 256, (x * y) % 256));
        }
    }
    const QString path = dir_.filePath(QStringLiteral("pattern.png"));
    QVERIFY(pattern.save(path) && window_->openDocumentAtPath(path));

    window_->registry()->refresh();
    QAction* action = window_->registry()->action(pictura::commandIdForPath(kCustomPath));
    QVERIFY(action != nullptr);
    QVERIFY(action->isEnabled());
    QCOMPARE(action->text(), QStringLiteral("Custom…"));
}

// A 5x5 grid of weights opening on pass-through, then Scale and Offset as bare
// value boxes, with a live preview.
void CustomFilterTest::dialogFollowsCs6()
{
    const pictura::FilterCommandSpec* spec = pictura::filterCommandForPath(kCustomPath);
    QVERIFY(spec != nullptr);
    QCOMPARE(spec->kind, QStringLiteral("custom"));
    QCOMPARE(static_cast<int>(pictura::filter_param_arity(spec->kind)), 27);
    pictura::FilterPreviewDialog dialog(window_->activeView(), *spec);
    QCOMPARE(dialog.windowTitle(), QStringLiteral("Custom"));
    QVERIFY(dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview")) != nullptr);
    QCOMPARE(dialog.values(), identity());

    const QList<QSpinBox*> cells = dialog.findChildren<QSpinBox*>(QStringLiteral("filterMatrixCell"));
    QCOMPARE(cells.size(), pictura::kMatrixCells);
    QCOMPARE(cells.first()->minimum(), -999);
    QCOMPARE(cells.first()->maximum(), 999);
    QVERIFY(dialog.findChildren<QSlider*>().isEmpty());
    const QList<QDoubleSpinBox*> boxes = dialog.findChildren<QDoubleSpinBox*>();
    QCOMPARE(boxes.size(), 2);
    QCOMPARE(boxes.at(0)->minimum(), 1.0);
    QCOMPARE(boxes.at(0)->maximum(), 9999.0);
    QCOMPARE(boxes.at(1)->minimum(), -9999.0);

    // Cells read row by row: the second row's first cell is slot 5.
    cells.at(5)->setValue(-3);
    QCOMPARE(dialog.values().at(5), -3.0);

    QList<double> sharpen(pictura::kMatrixCells, 0.0);
    sharpen[7] = sharpen[11] = sharpen[13] = sharpen[17] = -1.0;
    sharpen[12] = 5.0;
    sharpen << 2.0 << 10.0;
    dialog.applyInitial(sharpen);
    QCOMPARE(dialog.values(), sharpen);
    dialog.reject();
}

// Inverting through the grid (centre -1, Offset 255) previews on the canvas
// before OK and commits one history state.
void CustomFilterTest::menuOpensPreviewsAndCommits()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const QImage before = view->image();
    bool previewed = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = activeFilterDialog();
        if (!dialog) {
            return;
        }
        const QList<QSpinBox*> cells =
            dialog->findChildren<QSpinBox*>(QStringLiteral("filterMatrixCell"));
        const QList<QDoubleSpinBox*> boxes = dialog->findChildren<QDoubleSpinBox*>();
        if (cells.size() != pictura::kMatrixCells || boxes.size() != 2) {
            dialog->reject();
            return;
        }
        cells.at(pictura::kMatrixCells / 2)->setValue(-1);
        boxes.at(1)->setValue(255);
        QTRY_VERIFY_WITH_TIMEOUT(view->image().pixelColor(5, 9) != before.pixelColor(5, 9), 5000);
        previewed = true;
        dialog->accept();
    });
    const int states = view->history_count();
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kCustomPath)));
    QVERIFY(previewed);
    QCOMPARE(view->history_count(), states + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Filter: Custom"));
    QCOMPARE(filter_last_kind(*view), QStringLiteral("custom"));

    const QImage after = view->image();
    for (const QPoint p : {QPoint(0, 0), QPoint(5, 9), QPoint(31, 17)}) {
        const QColor was = before.pixelColor(p);
        const QColor now = after.pixelColor(p);
        QCOMPARE(now.red(), 255 - was.red());
        QCOMPARE(now.green(), 255 - was.green());
        QCOMPARE(now.blue(), 255 - was.blue());
    }
}

void CustomFilterTest::lastFilterReusesTheKernel()
{
    pictura::PictureView* view = window_->activeView();
    const QImage inverted = view->image();
    pictura::CommandRegistry* registry = window_->registry();
    registry->refresh();
    QAction* last = registry->action(QString::fromLatin1(pictura::command_ids::FilterLastFilter));
    QVERIFY(last != nullptr && last->isEnabled());
    QCOMPARE(last->text(), QStringLiteral("Custom"));
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::FilterLastFilter)));
    // Inverting twice is the original picture again.
    const QColor back = view->image().pixelColor(5, 9);
    const QColor once = inverted.pixelColor(5, 9);
    QCOMPARE(back.red(), 255 - once.red());
    QCOMPARE(back.green(), 255 - once.green());
}

QTEST_MAIN(CustomFilterTest)

#include "tst_custom_filter.moc"
