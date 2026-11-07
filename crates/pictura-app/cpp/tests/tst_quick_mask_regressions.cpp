#include "api/document_api.h"
#include "api/document_bridge.h"
#include "api/filters_bridge.h"
#include "api/history_api.h"
#include "api/layers_bridge.h"
#include "api/paint_bridge.h"
#include "api/selection_api.h"
#include "frame.h"
#include "image_view.h"
#include "panels/color_panel.h"
#include "qt_test_support.h"
#include "tools.h"
#include <QtTest/QtTest>
using namespace pictura;
class QuickMaskRegressionsTest : public QObject {
    Q_OBJECT
private slots:
    void init();
    void cleanup();
    void entryHidesCachedAnts();
    void editingSelectionOnlyClipsMask();
    void keyboardUndoRepaintsMask();
    void editingSelectionHistorySurvivesExit();
    void brushRetainsExistingMask();
    void strokesUseBoundedPresentation();
    void zoomedOutStrokesPatchCache();
    void geometryResetDoesNotLeakStaleCoverage();

private:
    void stroke(int x, int y)
    {
        auto* canvas = window_->imageView();
        canvas->mousePressed(QPointF(x, y), Qt::LeftButton, int(Qt::NoModifier));
        canvas->mouseMoved(QPointF(x + 1, y));
        canvas->mouseReleased(QPointF(x + 1, y));
    }
    QColor displayed(int x, int y)
    {
        auto* canvas = window_->imageView();
        const QPoint p = canvas->imageToWidget(QPointF(x, y)).toPoint();
        return canvas->grab().toImage().pixelColor(p);
    }
    int mask(int x, int y) { return quick_mask_coverage(*view_, x, y); }
    test::ScopedStateHome stateHome_;
    std::unique_ptr<PicturaMainWindow> window_;
    PictureView* view_ = nullptr;
};
void QuickMaskRegressionsTest::init()
{
    QVERIFY(stateHome_.isValid());
    window_ = test::makeMainWindow();
    QVERIFY(window_->newDocument(QStringLiteral("Mask regressions"), 64, 64,
        QStringLiteral("rgb"), 8, QStringLiteral("white")));
    view_ = window_->activeView();
    view_->set_gpu_compute(false);
    const auto layer = view_->add_layer_in(QString());
    window_->selectLayerPath(layer);
    window_->show();
    window_->setActiveTool(ToolId::Brush);
    auto* options = window_->findChild<ToolController*>()->optionsFor<PaintOptions>(ToolId::Brush);
    options->setSize(8);
    options->setHardness(100);
    QCoreApplication::processEvents();
}
void QuickMaskRegressionsTest::cleanup()
{
    while (window_->activeDocumentIndex() >= 0)
        window_->closeDocument(window_->activeDocumentIndex(), false);
    window_.reset();
}
void QuickMaskRegressionsTest::entryHidesCachedAnts()
{
    QVERIFY(view_->select_rect(16, 16, 32, 32, QStringLiteral("new"), 0));
    auto* canvas = window_->imageView();
    QVERIFY(canvas->hasSelectionContourForTest());
    QTest::keyClick(canvas, Qt::Key_Q);
    QVERIFY(quick_mask_active(*view_));
    QVERIFY(!canvas->hasSelectionContourForTest());
    QCOMPARE(mask(24, 24), 255);
    QCOMPARE(mask(8, 8), 0);
    QVERIFY(view_->select_rect(20, 20, 8, 8, QStringLiteral("new"), 0));
    QVERIFY(canvas->hasSelectionContourForTest());
}
void QuickMaskRegressionsTest::editingSelectionOnlyClipsMask()
{
    QVERIFY(view_->select_rect(16, 16, 32, 32, QStringLiteral("new"), 0));
    const QImage pixels = view_->image();
    QVERIFY(quick_mask_set(*view_, true));
    QVERIFY(view_->select_rect(20, 20, 8, 8, QStringLiteral("new"), 0));
    QVERIFY(bucket_fill_at(*view_, 22, 22, 0xff000000u, -1, QStringLiteral("normal"), 100, 0, false, true, false));
    QCOMPARE(mask(24, 24), 0);
    QCOMPARE(mask(40, 40), 255);
    QCOMPARE(mask(8, 8), 0);
    auto* colors = window_->findChild<ColorState*>();
    colors->setForeground(Qt::white);
    stroke(24, 24);
    QCOMPARE(mask(24, 24), 255);
    stroke(8, 8);
    QCOMPARE(mask(8, 8), 0);
    QVERIFY(filter_preview(*view_, QStringLiteral("gaussian-blur"), { 3.0 }));
    QCOMPARE(mask(40, 40), 255);
    QCOMPARE(mask(17, 32), 255);
    QCOMPARE(mask(8, 8), 0);
    QVERIFY(filter_preview_cancel(*view_));
    view_->deselect();
    QVERIFY(!view_->has_selection());
    QCOMPARE(mask(40, 40), 255);
    QCOMPARE(mask(8, 8), 0);
    QVERIFY(view_->select_rect(1, 1, 4, 4, QStringLiteral("new"), 0));
    QTest::keyClick(window_->imageView(), Qt::Key_Q);
    QVERIFY(!quick_mask_active(*view_));
    QCOMPARE(view_->selection_coverage(40, 40), 255);
    QCOMPARE(view_->selection_coverage(2, 2), 0);
    QCOMPARE(view_->image(), pixels);
}
void QuickMaskRegressionsTest::keyboardUndoRepaintsMask()
{
    QVERIFY(view_->select_rect(16, 16, 32, 32, QStringLiteral("new"), 0));
    QVERIFY(quick_mask_set(*view_, true));
    QCOMPARE(displayed(32, 32), QColor(Qt::white));
    const int history = view_->history_index();
    stroke(32, 32);
    QCOMPARE(displayed(32, 32), QColor(255, 128, 128));
    QCOMPARE(view_->history_index(), history + 1);
    QTest::keyClick(window_->imageView(), Qt::Key_Z, Qt::ControlModifier);
    QCOMPARE(view_->history_index(), history);
    QCOMPARE(mask(32, 32), 255);
    QCOMPARE(displayed(32, 32), QColor(Qt::white));
    QTest::keyClick(window_->imageView(), Qt::Key_Y, Qt::ControlModifier);
    QCOMPARE(view_->history_index(), history + 1);
    QCOMPARE(mask(32, 32), 0);
    QCOMPARE(displayed(32, 32), QColor(255, 128, 128));
}
void QuickMaskRegressionsTest::editingSelectionHistorySurvivesExit()
{
    QVERIFY(quick_mask_set(*view_, true));
    QVERIFY(view_->select_rect(16, 16, 32, 32, QStringLiteral("new"), 0));
    stroke(32, 32);
    QVERIFY(view_->undo());
    QCOMPARE(mask(32, 32), 255);
    QVERIFY(view_->has_selection());
    QVERIFY(view_->undo());
    QVERIFY(!view_->has_selection());
    QVERIFY(view_->redo());
    QVERIFY(view_->has_selection());
    QVERIFY(view_->redo());
    QCOMPARE(mask(32, 32), 0);
    QVERIFY(quick_mask_set(*view_, false));
    QCOMPARE(view_->selection_coverage(4, 4), 255);
    QVERIFY(view_->undo());
    QVERIFY(!view_->has_selection());
    QVERIFY(view_->redo());
    QCOMPARE(view_->selection_coverage(4, 4), 255);
    QCOMPARE(view_->selection_coverage(32, 32), 0);
}
void QuickMaskRegressionsTest::brushRetainsExistingMask()
{
    QVERIFY(view_->select_rect(16, 16, 32, 32, QStringLiteral("new"), 0));
    const QImage pixels = view_->image();
    QVERIFY(quick_mask_set(*view_, true));
    stroke(24, 24);
    QCOMPARE(mask(24, 24), 0);
    QCOMPARE(mask(40, 40), 255);
    QCOMPARE(mask(8, 8), 0);
    stroke(40, 40);
    QCOMPARE(mask(24, 24), 0);
    QCOMPARE(mask(40, 40), 0);
    QCOMPARE(mask(32, 32), 255);
    QCOMPARE(mask(8, 8), 0);
    QCOMPARE(view_->image(), pixels);
}
void QuickMaskRegressionsTest::strokesUseBoundedPresentation()
{
    QVERIFY(quick_mask_set(*view_, true));
    auto* canvas = window_->imageView();
    QSignalSpy changed(view_, &PictureView::changed);
    QSignalSpy damaged(view_, &PictureView::regionDamaged);
    const auto frame = frame_revision(*view_);
    canvas->mousePressed(QPointF(24, 24), Qt::LeftButton, int(Qt::NoModifier));
    QVERIFY(view_->is_painting());
    canvas->mouseMoved(QPointF(28, 24));
    view_->flush_present();
    QVERIFY(!damaged.isEmpty());
    QCOMPARE(changed.count(), 0);
    QCOMPARE(frame_revision(*view_), frame);
    for (const auto& args : damaged) {
        QVERIFY(args[2].toInt() < 64);
        QVERIFY(args[3].toInt() < 64);
    }
    canvas->mouseReleased(QPointF(28, 24));
    QVERIFY(!view_->is_painting());
    QVERIFY(changed.count() > 0);
}
void QuickMaskRegressionsTest::zoomedOutStrokesPatchCache()
{
    QVERIFY(window_->newDocument(QStringLiteral("Zoomed mask"), 512, 512,
        QStringLiteral("rgb"), 8, QStringLiteral("white")));
    view_ = window_->activeView();
    view_->set_gpu_compute(false);
    const auto layer = view_->add_layer_in(QString());
    window_->selectLayerPath(layer);
    QVERIFY(quick_mask_set(*view_, true));
    auto* canvas = window_->imageView();
    canvas->setZoom(0.25, QPointF(canvas->width() / 2, canvas->height() / 2));
    canvas->grab();
    QVERIFY(canvas->presentLevelForTest() > 0);
    QVERIFY(canvas->presentCacheValidForTest());
    const int rebuilds = canvas->presentCacheRebuildCount();
    canvas->mousePressed(QPointF(256, 256), Qt::LeftButton, int(Qt::NoModifier));
    canvas->grab();
    QCOMPARE(canvas->presentCacheRebuildCount(), rebuilds);
    canvas->mouseMoved(QPointF(260, 256));
    view_->flush_present();
    canvas->grab();
    QCOMPARE(canvas->presentCacheRebuildCount(), rebuilds);
    // At this zoom the brush ring overlaps the center pixel.
    canvas->clearBrushOutline();
    QCOMPARE(displayed(256, 256), QColor(255, 128, 128));
    canvas->mouseReleased(QPointF(260, 256));
}
void QuickMaskRegressionsTest::geometryResetDoesNotLeakStaleCoverage()
{
    QVERIFY(view_->select_rect(16, 16, 32, 32, QStringLiteral("new"), 0));
    for (int op = 0; op < 4; ++op) {
        QVERIFY(quick_mask_set(*view_, true));
        QVERIFY(view_->select_rect(20, 20, 8, 8, QStringLiteral("new"), 0));
        switch (op) {
        case 0:
            QVERIFY(view_->resize_image(QStringLiteral("nearest"), 128, 96));
            break;
        case 1:
            QVERIFY(view_->resize_canvas(QStringLiteral("center"), 80, 96));
            break;
        case 2:
            QVERIFY(view_->rotate_doc(1));
            break;
        case 3:
            QVERIFY(view_->crop(8, 8, 32, 32));
            break;
        }
        QVERIFY(!view_->has_selection());
        QCOMPARE(mask(20, 20), 255);
        QVERIFY(quick_mask_set(*view_, false));
        QVERIFY(!view_->has_selection());
        QVERIFY(view_->undo());
        QCOMPARE(view_->document_width(), 64);
        QCOMPARE(view_->document_height(), 64);
        QCOMPARE(view_->selection_coverage(40, 40), 255);
        QCOMPARE(view_->selection_coverage(8, 8), 0);
    }
}
QTEST_MAIN(QuickMaskRegressionsTest)
#include "tst_quick_mask_regressions.moc"
