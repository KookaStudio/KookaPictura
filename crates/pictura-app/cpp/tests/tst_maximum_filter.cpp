#include <QtTest/QtTest>

#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QLabel>

#include "commands.h"
#include "filter_commands.h"
#include "filter_preview_dialog.h"
#include "frame.h"
#include "api/filters_bridge.h"

#include "qt_test_support.h"

namespace {

const QStringList kMaximumPath = {QStringLiteral("Filter"), QStringLiteral("Other"),
                                  QStringLiteral("Maximum")};
const QStringList kMinimumPath = {QStringLiteral("Filter"), QStringLiteral("Other"),
                                  QStringLiteral("Minimum")};

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

class MaximumFilterTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void dialogFollowsCs6();
    void warningAppearsAboveOneHundred();
    void roundnessCommitsAndRepeats();

private:
    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void MaximumFilterTest::initTestCase()
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
}

// Both descriptors carry a 1..=500 radius and a Preserve menu opening on
// Squareness.
void MaximumFilterTest::dialogFollowsCs6()
{
    for (const auto& entry : {qMakePair(kMaximumPath, QStringLiteral("maximum")),
                              qMakePair(kMinimumPath, QStringLiteral("minimum"))}) {
        const pictura::FilterCommandSpec* spec = pictura::filterCommandForPath(entry.first);
        QVERIFY(spec != nullptr);
        QCOMPARE(spec->kind, entry.second);
        QCOMPARE(static_cast<int>(pictura::filter_param_arity(spec->kind)), 2);
        QCOMPARE(spec->params.size(), 2);
        QCOMPARE(spec->params.at(0).maximum, 500.0);
        QCOMPARE(spec->params.at(1).choices,
                 QStringList({QStringLiteral("Squareness"), QStringLiteral("Roundness")}));
        QCOMPARE(spec->params.at(1).value, 0.0);

        pictura::FilterPreviewDialog dialog(window_->activeView(), *spec);
        QCOMPARE(dialog.values(), QList<double>({2.0, 0.0}));

        const QList<QDoubleSpinBox*> boxes = dialog.findChildren<QDoubleSpinBox*>();
        QCOMPARE(boxes.size(), 1);
        QCOMPARE(boxes.first()->minimum(), 1.0);
        QCOMPARE(boxes.first()->maximum(), 500.0);

        const QList<QComboBox*> menus = dialog.findChildren<QComboBox*>();
        QCOMPARE(menus.size(), 1);
        QCOMPARE(menus.first()->count(), 2);
        QCOMPARE(menus.first()->itemText(0), QStringLiteral("Squareness"));
        QCOMPARE(menus.first()->itemText(1), QStringLiteral("Roundness"));
        QCOMPARE(menus.first()->currentIndex(), 0);

        auto* warning = dialog.findChild<QLabel*>(QStringLiteral("filterWarning"));
        QVERIFY(warning != nullptr);
        QVERIFY(warning->isHidden());
        dialog.reject();
    }
}

// The warning is inline and non-blocking: it appears above 100 and hides at
// the threshold.
void MaximumFilterTest::warningAppearsAboveOneHundred()
{
    const pictura::FilterCommandSpec* spec = pictura::filterCommandForPath(kMaximumPath);
    QVERIFY(spec != nullptr);
    pictura::FilterPreviewDialog dialog(window_->activeView(), *spec);
    auto* warning = dialog.findChild<QLabel*>(QStringLiteral("filterWarning"));
    QVERIFY(warning != nullptr);

    auto* radius = dialog.findChild<QDoubleSpinBox*>();
    QVERIFY(radius != nullptr);
    radius->setValue(150);
    QVERIFY(!warning->isHidden());
    QVERIFY(!warning->text().isEmpty());

    radius->setValue(100);
    QVERIFY(warning->isHidden());
    dialog.reject();
}

// Choosing Roundness previews, commits one state carrying slot 1, and Last
// Filter repeats the same radius and choice.
void MaximumFilterTest::roundnessCommitsAndRepeats()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const QImage before = view->image();
    bool changed = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = activeFilterDialog();
        if (!dialog) {
            return;
        }
        auto* menu = dialog->findChild<QComboBox*>();
        auto* radius = dialog->findChild<QDoubleSpinBox*>();
        if (!menu || !radius) {
            dialog->reject();
            return;
        }
        menu->setCurrentIndex(1);
        radius->setValue(4);
        changed = true;
        dialog->accept();
    });
    const int states = view->history_count();
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kMaximumPath)));
    QVERIFY(changed);
    QCOMPARE(view->history_count(), states + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Filter: Maximum"));
    QCOMPARE(pictura::filter_last_kind(*view), QStringLiteral("maximum"));
    QCOMPARE(pictura::filter_last_params(*view), QList<double>({4.0, 1.0}));
    QVERIFY(view->image() != before);

    window_->registry()->refresh();
    QAction* last =
        window_->registry()->action(QString::fromLatin1(pictura::command_ids::FilterLastFilter));
    QVERIFY(last != nullptr && last->isEnabled());
    QCOMPARE(last->text(), QStringLiteral("Maximum"));
    const int afterFirst = view->history_count();
    QVERIFY(window_->registry()->dispatch(QString::fromLatin1(pictura::command_ids::FilterLastFilter)));
    QCOMPARE(view->history_count(), afterFirst + 1);
}

QTEST_MAIN(MaximumFilterTest)

#include "tst_maximum_filter.moc"
