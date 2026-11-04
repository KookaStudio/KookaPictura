// Select > Color Range (#67): the dialog previews the coverage mask, samples
// from a canvas click, and OK selects (or narrows a live selection).

#include <QtTest/QtTest>

#include "color_range_dialog.h"
#include "commands.h"
#include "frame.h"
#include "image_view.h"
#include "api/layers_bridge.h"

#include <QtGui/QAction>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QPushButton>

#include "qt_test_support.h"

class ColorRangeTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void sampleAndSelect();
    void refinesALiveSelection();

private:
    // A white 40x40 document with a red square over (2, 2)-(10, 10).
    bool setupRedSquare();
    pictura::ColorRangeDialog* open();
    QAction* command() const;

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
};

void ColorRangeTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
}

void ColorRangeTest::cleanup()
{
    for (auto* dialog : window_->findChildren<pictura::ColorRangeDialog*>()) {
        dialog->reject();
    }
    QCoreApplication::processEvents();
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    view_ = nullptr;
}

bool ColorRangeTest::setupRedSquare()
{
    if (!window_->newDocument(QStringLiteral("Range"), 40, 40, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return false;
    }
    view_ = window_->activeView();
    pictura::ShapeSpec square{};
    square.kind = 0;
    square.boxed = true;
    square.x0 = 2;
    square.y0 = 2;
    square.x1 = 10;
    square.y1 = 10;
    return view_ && !pictura::shape_add_layer(*view_, square, 0xffff0000u).isEmpty()
        && view_->sample_argb(5, 5) == 0xffff0000u;
}

QAction* ColorRangeTest::command() const
{
    return window_->registry()->action(
        pictura::commandIdForPath({QStringLiteral("Select"), QStringLiteral("Color Range…")}));
}

pictura::ColorRangeDialog* ColorRangeTest::open()
{
    QAction* action = command();
    window_->registry()->refresh();
    if (!action || !action->isEnabled()) {
        return nullptr;
    }
    action->trigger();
    QCoreApplication::processEvents();
    return window_->findChild<pictura::ColorRangeDialog*>();
}

void ColorRangeTest::sampleAndSelect()
{
    QVERIFY(setupRedSquare());
    pictura::ToolController* tools = window_->findChild<pictura::ToolController*>();
    pictura::ColorRangeDialog* dialog = open();
    QVERIFY(dialog && dialog->isVisible() && tools);
    QVERIFY(tools->canvasSamplerActive());

    // A canvas click samples the red instead of reaching the active tool.
    const int before = view_->history_count();
    emit window_->imageView()->mousePressed(QPointF(5, 5), Qt::LeftButton, 0);
    QCOMPARE(dialog->sampledColor(), QColor(Qt::red));
    QCOMPARE(view_->history_count(), before);

    // The preview is the mask: white over the square, black elsewhere.
    QImage preview = dialog->previewForTest();
    QCOMPARE(preview.size(), QSize(40, 40));
    QCOMPARE(qGray(preview.pixel(5, 5)), 255);
    QCOMPARE(qGray(preview.pixel(30, 30)), 0);

    // A tonal band needs no sample: Highlights takes the white, not the red.
    auto* select = dialog->findChild<QComboBox*>(QStringLiteral("colorRangeSelect"));
    QVERIFY(select);
    select->setCurrentIndex(select->findData(7));
    QVERIFY(!tools->canvasSamplerActive());
    preview = dialog->previewForTest();
    QVERIFY(qGray(preview.pixel(30, 30)) > 200);
    QVERIFY(qGray(preview.pixel(5, 5)) < 100);
    select->setCurrentIndex(select->findData(0));
    QVERIFY(tools->canvasSamplerActive());

    // OK selects the red square as one "Color Range" state.
    dialog->findChild<QDialogButtonBox*>()->button(QDialogButtonBox::Ok)->click();
    QCoreApplication::processEvents();
    QCOMPARE(view_->history_count(), before + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Color Range"));
    QCOMPARE(view_->selection_coverage(5, 5), 255);
    QCOMPARE(view_->selection_coverage(30, 30), 0);
    QVERIFY(!tools->canvasSamplerActive());

    // Cancel changes nothing.
    dialog = open();
    QVERIFY(dialog);
    const int after = view_->history_count();
    dialog->reject();
    QCoreApplication::processEvents();
    QCOMPARE(view_->history_count(), after);
}

void ColorRangeTest::refinesALiveSelection()
{
    QVERIFY(setupRedSquare());
    // Run on a selection, Color Range narrows it to the matching subset.
    QVERIFY(view_->select_rect(0, 0, 6, 40, QStringLiteral("new"), 0.0));
    pictura::ColorRangeDialog* dialog = open();
    QVERIFY(dialog);
    dialog->sampleAt(QPointF(5, 5));
    dialog->accept();
    QCoreApplication::processEvents();
    QCOMPARE(view_->selection_coverage(4, 4), 255);
    QCOMPARE(view_->selection_coverage(8, 8), 0);
    QCOMPARE(view_->selection_coverage(3, 30), 0);
}

QTEST_MAIN(ColorRangeTest)
#include "tst_color_range.moc"
