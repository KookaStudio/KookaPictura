#include <QtTest/QtTest>

#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QListWidget>

#include "commands.h"
#include "filter_commands.h"
#include "filter_preview_dialog.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/shapes.cxxqt.h"

#include "qt_test_support.h"

namespace {

const QStringList kShapeBlurPath = {QStringLiteral("Filter"), QStringLiteral("Blur"),
                                    QStringLiteral("Shape Blur")};

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

class ShapeBlurTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void dialogFollowsCs6();
    void menuOpensPreviewsAndCommits();
    void lastFilterReusesTheShape();

private:
    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void ShapeBlurTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);

    // Hard-edged stripes, so any blur moves the samples beside each edge.
    QImage pattern(48, 48, QImage::Format_RGB32);
    for (int y = 0; y < 48; ++y) {
        for (int x = 0; x < 48; ++x) {
            const bool on = (x / 6 + y / 6) % 2 == 0;
            pattern.setPixelColor(x, y, on ? QColor(240, 200, 30) : QColor(20, 40, 160));
        }
    }
    const QString path = dir_.filePath(QStringLiteral("stripes.png"));
    QVERIFY(pattern.save(path) && window_->openDocumentAtPath(path));

    window_->registry()->refresh();
    QAction* action = window_->registry()->action(pictura::commandIdForPath(kShapeBlurPath));
    QVERIFY(action != nullptr);
    QVERIFY(action->isEnabled());
    QCOMPARE(action->text(), QStringLiteral("Shape Blur…"));
}

// Radius 5..1000 over a grid of every Custom Shape, opening on radius 10 and
// the first shape, with a live preview.
void ShapeBlurTest::dialogFollowsCs6()
{
    const pictura::FilterCommandSpec* spec = pictura::filterCommandForPath(kShapeBlurPath);
    QVERIFY(spec != nullptr);
    QCOMPARE(spec->kind, QStringLiteral("shape-blur"));
    QCOMPARE(static_cast<int>(pictura::filter_param_arity(spec->kind)), 2);
    pictura::FilterPreviewDialog dialog(window_->activeView(), *spec);
    QCOMPARE(dialog.windowTitle(), QStringLiteral("Shape Blur"));
    QVERIFY(dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview")) != nullptr);
    QCOMPARE(dialog.values(), (QList<double>{10.0, 0.0}));

    const QList<QDoubleSpinBox*> boxes = dialog.findChildren<QDoubleSpinBox*>();
    QCOMPARE(boxes.size(), 1);
    QCOMPARE(boxes.first()->minimum(), 5.0);
    QCOMPARE(boxes.first()->maximum(), 1000.0);
    auto* grid = dialog.findChild<QListWidget*>(QStringLiteral("filterShapeGrid"));
    QVERIFY(grid != nullptr);
    QCOMPARE(grid->count(), pictura::shape_custom_count());
    QCOMPARE(grid->item(1)->toolTip(), QStringLiteral("Heart"));
    QVERIFY(!grid->item(0)->icon().isNull());

    grid->setCurrentRow(2);
    QCOMPARE(dialog.values().at(1), 2.0);
    dialog.applyInitial({46.0, 1.0});
    QCOMPARE(dialog.values(), (QList<double>{46.0, 1.0}));
    QCOMPARE(grid->currentRow(), 1);
    dialog.reject();
}

// Picking the heart and a radius previews on the canvas before OK and commits
// one history state.
void ShapeBlurTest::menuOpensPreviewsAndCommits()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const QImage before = view->image();
    const QPoint edge(6, 3);
    bool previewed = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = activeFilterDialog();
        if (!dialog) {
            return;
        }
        auto* grid = dialog->findChild<QListWidget*>(QStringLiteral("filterShapeGrid"));
        const QList<QDoubleSpinBox*> boxes = dialog->findChildren<QDoubleSpinBox*>();
        if (!grid || boxes.size() != 1) {
            dialog->reject();
            return;
        }
        grid->setCurrentRow(1);
        boxes.first()->setValue(6);
        QTRY_VERIFY_WITH_TIMEOUT(view->image().pixelColor(edge) != before.pixelColor(edge), 5000);
        previewed = true;
        dialog->accept();
    });
    const int states = view->history_count();
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kShapeBlurPath)));
    QVERIFY(previewed);
    QCOMPARE(view->history_count(), states + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Filter: Shape Blur"));
    QCOMPARE(filter_last_kind(*view), QStringLiteral("shape-blur"));
    QVERIFY(view->image().pixelColor(edge) != before.pixelColor(edge));
}

void ShapeBlurTest::lastFilterReusesTheShape()
{
    pictura::PictureView* view = window_->activeView();
    const QImage once = view->image();
    pictura::CommandRegistry* registry = window_->registry();
    registry->refresh();
    QAction* last = registry->action(QString::fromLatin1(pictura::command_ids::FilterLastFilter));
    QVERIFY(last != nullptr && last->isEnabled());
    QCOMPARE(last->text(), QStringLiteral("Shape Blur"));
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::FilterLastFilter)));
    QVERIFY(view->image() != once);
}

QTEST_MAIN(ShapeBlurTest)

#include "tst_shape_blur.moc"
