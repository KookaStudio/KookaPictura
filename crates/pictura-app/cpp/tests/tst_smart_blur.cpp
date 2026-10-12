#include <QtTest/QtTest>

#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>

#include "commands.h"
#include "filter_commands.h"
#include "filter_preview_dialog.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"

#include "qt_test_support.h"

namespace {

const QStringList kSmartBlurPath = {QStringLiteral("Filter"), QStringLiteral("Blur"),
                                    QStringLiteral("Smart Blur")};

pictura::FilterPreviewDialog* activeFilterDialog()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<pictura::FilterPreviewDialog*>(widget)) {
            return dialog;
        }
    }
    return nullptr;
}

bool blackOrWhite(const QColor& c)
{
    return (c.red() == 0 && c.green() == 0 && c.blue() == 0)
           || (c.red() == 255 && c.green() == 255 && c.blue() == 255);
}

} // namespace

class SmartBlurTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void dialogFollowsCs6();
    void edgeOnlyPreviewsAndCommits();
    void lastFilterRepeatsTheMode();

private:
    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void SmartBlurTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);

    // Two flat halves split at x = 24, far apart in every channel.
    QImage halves(48, 48, QImage::Format_RGB32);
    for (int y = 0; y < 48; ++y) {
        for (int x = 0; x < 48; ++x) {
            halves.setPixelColor(x, y, x < 24 ? QColor(220, 90, 120) : QColor(30, 110, 40));
        }
    }
    const QString path = dir_.filePath(QStringLiteral("halves.png"));
    QVERIFY(halves.save(path) && window_->openDocumentAtPath(path));

    window_->registry()->refresh();
    QAction* action = window_->registry()->action(pictura::commandIdForPath(kSmartBlurPath));
    QVERIFY(action != nullptr);
    QVERIFY(action->isEnabled());
    QCOMPARE(action->text(), QStringLiteral("Smart Blur…"));
}

// Radius and Threshold 0.1..100 to one decimal, Quality Low/Medium/High and
// Mode Normal/Edge Only/Overlay Edge, with a live preview.
void SmartBlurTest::dialogFollowsCs6()
{
    const pictura::FilterCommandSpec* spec = pictura::filterCommandForPath(kSmartBlurPath);
    QVERIFY(spec != nullptr);
    QCOMPARE(spec->kind, QStringLiteral("smart-blur"));
    QCOMPARE(static_cast<int>(pictura::filter_param_arity(spec->kind)), 4);
    pictura::FilterPreviewDialog dialog(window_->activeView(), *spec);
    QCOMPARE(dialog.windowTitle(), QStringLiteral("Smart Blur"));
    QVERIFY(dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview")) != nullptr);
    QCOMPARE(dialog.values(), (QList<double>{3.0, 25.0, 0.0, 0.0}));

    const QList<QDoubleSpinBox*> boxes = dialog.findChildren<QDoubleSpinBox*>();
    QCOMPARE(boxes.size(), 2);
    for (QDoubleSpinBox* box : boxes) {
        QCOMPARE(box->minimum(), 0.1);
        QCOMPARE(box->maximum(), 100.0);
        QCOMPARE(box->decimals(), 1);
    }
    const QList<QComboBox*> combos = dialog.findChildren<QComboBox*>();
    QCOMPARE(combos.size(), 2);
    QStringList items;
    for (QComboBox* combo : combos) {
        for (int i = 0; i < combo->count(); ++i) {
            items << combo->itemText(i);
        }
    }
    QCOMPARE(items, (QStringList{QStringLiteral("Low"), QStringLiteral("Medium"),
                                 QStringLiteral("High"), QStringLiteral("Normal"),
                                 QStringLiteral("Edge Only"), QStringLiteral("Overlay Edge")}));
    dialog.reject();
}

// Edge Only previews a black-and-white map before OK and commits one state:
// white along the split, black across the flat halves.
void SmartBlurTest::edgeOnlyPreviewsAndCommits()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const QPoint flat(6, 20);
    const QPoint edge(24, 20);
    bool previewed = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = activeFilterDialog();
        if (!dialog) {
            return;
        }
        const QList<QComboBox*> combos = dialog->findChildren<QComboBox*>();
        if (combos.size() != 2) {
            dialog->reject();
            return;
        }
        combos.at(1)->setCurrentIndex(1);
        QTRY_VERIFY_WITH_TIMEOUT(view->image().pixelColor(flat) == QColor(0, 0, 0), 5000);
        previewed = true;
        dialog->accept();
    });
    const int states = view->history_count();
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kSmartBlurPath)));
    QVERIFY(previewed);
    QCOMPARE(view->history_count(), states + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Filter: Smart Blur"));
    QCOMPARE(filter_last_kind(*view), QStringLiteral("smart-blur"));
    const QImage after = view->image();
    QCOMPARE(after.pixelColor(flat), QColor(0, 0, 0));
    QCOMPARE(after.pixelColor(edge), QColor(255, 255, 255));
    for (int x = 0; x < after.width(); ++x) {
        QVERIFY(blackOrWhite(after.pixelColor(x, 30)));
    }
}

void SmartBlurTest::lastFilterRepeatsTheMode()
{
    pictura::PictureView* view = window_->activeView();
    pictura::CommandRegistry* registry = window_->registry();
    registry->refresh();
    QAction* last = registry->action(QString::fromLatin1(pictura::command_ids::FilterLastFilter));
    QVERIFY(last != nullptr && last->isEnabled());
    QCOMPARE(last->text(), QStringLiteral("Smart Blur"));
    const int states = view->history_count();
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::FilterLastFilter)));
    QCOMPARE(view->history_count(), states + 1);
    // Edge Only of a white line on black outlines both of its sides.
    const QImage again = view->image();
    QCOMPARE(again.pixelColor(22, 20), QColor(255, 255, 255));
    QCOMPARE(again.pixelColor(6, 20), QColor(0, 0, 0));
}

QTEST_MAIN(SmartBlurTest)

#include "tst_smart_blur.moc"
