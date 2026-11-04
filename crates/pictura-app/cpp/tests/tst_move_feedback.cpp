// Move tool feedback: Show Transform Controls starts checked, a Move drag
// draws marching ants round the moving content with a ΔX / ΔY readout in the
// rulers' unit, switching to the tool builds no preview, and a layer too large
// to bake whole previews from the view (#1148).

#include <QtTest/QtTest>

#include "api/transform_api.h"
#include "api/type_api.h"
#include "canvas_ruler.h"
#include "selftest_paint_fixture.h"

#include "api/document_bridge.h"
#include "api/transform_bridge.h"

#include <memory>

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

// 200 × 160 white at 72 ppi (2835 dots per metre).
QImage seed()
{
    QImage image(200, 160, QImage::Format_RGB32);
    image.fill(Qt::white);
    image.setDotsPerMeterX(2835);
    image.setDotsPerMeterY(2835);
    return image;
}

QString typeLayer(const Fixture& f)
{
    for (int i = 0; i < f.view->layer_count(); ++i) {
        if (pictura::type_api::readLayer(*f.view, QString::number(i)).isType) {
            return QString::number(i);
        }
    }
    return QString();
}

} // namespace

class MoveFeedbackTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void transformControlsOnByDefault();
    void dragShowsAntsAndReadout();
    void readoutFollowsTheRulerUnit();
    void switchingToMoveBuildsNoPreview();
    void largeLayerPreviewsFromTheView();

private:
    void showFrame();

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void MoveFeedbackTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void MoveFeedbackTest::showFrame()
{
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    window_->activateWindow();
    QCoreApplication::processEvents();
}

void MoveFeedbackTest::transformControlsOnByDefault()
{
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(tools);
    const pictura::MoveGroup* move = tools->optionsFor<pictura::MoveGroup>(pictura::ToolId::Move);
    QVERIFY(move);
    QVERIFY(move->get().showTransformControls);
}

void MoveFeedbackTest::dragShowsAntsAndReadout()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_move_feedback"));
    QVERIFY2(f.ok(), "move fixture");
    showFrame();
    QVERIFY2(qAbs(pictura::document_ppi(*f.view) - 72.0) < 0.1, "72 ppi");
    // This check is about the ants and readout, not snapping.
    f.canvas->setSmartGuidesEnabled(false);

    frame.setActiveTool(pictura::ToolId::HorizontalType);
    f.drag({QPointF(20, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("Move me"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY2(!typeLayer(f).isEmpty(), "a type layer");
    frame.setActiveTool(pictura::ToolId::Move);

    // Press off the transform box: an ordinary Move drag of the active layer.
    f.canvas->mousePressed(QPointF(150, 140), Qt::LeftButton, int(Qt::NoModifier));
    QVERIFY(f.canvas->movePreviewActive());
    const QRectF start = f.canvas->moveOutlineForTest();
    QVERIFY2(!start.isEmpty() && start.left() >= 19.0 && start.top() < 60.0
                 && start.bottom() > 55.0,
             "the ants box the text");
    f.canvas->mouseMoved(QPointF(170, 150));
    f.canvas->mouseMoved(QPointF(186, 158));
    const QRectF moved = f.canvas->moveOutlineForTest();
    QCOMPARE(moved.topLeft() - start.topLeft(), QPointF(36, 18));
    QCOMPARE(moved.size(), start.size());
    QVERIFY(f.canvas->hasDragSizeHintForTest());
    QCOMPARE(f.canvas->dragSizeHintForTest(), QStringLiteral("ΔX: 0.500 in\nΔY: 0.250 in"));

    f.canvas->mouseReleased(QPointF(186, 158));
    QVERIFY(!f.canvas->hasDragSizeHintForTest());
    QVERIFY(f.canvas->moveOutlineForTest().isEmpty());
}

void MoveFeedbackTest::readoutFollowsTheRulerUnit()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_move_units"));
    QVERIFY2(f.ok(), "move fixture");
    const QPointF offset(36, 72);
    f.canvas->setMeasureUnit(pictura::RulerUnit::Pixels, false);
    QCOMPARE(f.canvas->offsetReadout(offset), QStringLiteral("ΔX: 36 px\nΔY: 72 px"));
    f.canvas->setMeasureUnit(pictura::RulerUnit::Centimeters, false);
    QCOMPARE(f.canvas->offsetReadout(offset), QStringLiteral("ΔX: 1.27 cm\nΔY: 2.54 cm"));
    f.canvas->setMeasureUnit(pictura::RulerUnit::Percent, false);
    QCOMPARE(f.canvas->offsetReadout(offset), QStringLiteral("ΔX: 18.0%\nΔY: 45.0%"));
    f.canvas->setMeasureUnit(pictura::RulerUnit::Inches, false);
}

// [mvp_no_activation_bake]
void MoveFeedbackTest::switchingToMoveBuildsNoPreview()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_move_activate"));
    QVERIFY2(f.ok(), "move fixture");
    QCOMPARE(f.view->duplicate_layer(0), 1);
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    frame.setActiveTool(pictura::ToolId::Move);
    QVERIFY2(f.view->move_preview_base().isNull(), "switching to the tool builds nothing");

    // An undo re-applies the tool policy; it builds nothing either.
    QVERIFY(f.view->add_layer(1) >= 0);
    QVERIFY(f.view->undo());
    QCoreApplication::processEvents();
    QVERIFY2(f.view->move_preview_base().isNull(), "an undo builds nothing");

    QVERIFY(f.view->begin_move_preview());
    QVERIFY2(!f.view->move_preview_cache_hit(), "the press builds the preview");
    QVERIFY(!f.view->move_preview_base().isNull());
    f.view->end_move_preview();
}

// [mvp_view_preview]
void MoveFeedbackTest::largeLayerPreviewsFromTheView()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage big(2400, 2000, QImage::Format_RGB32);
    big.fill(Qt::white);
    Fixture f(frame, big, QStringLiteral("pictura_move_view"));
    QVERIFY2(f.ok(), "large move fixture");
    showFrame();
    f.canvas->setSmartGuidesEnabled(false);
    QCOMPARE(f.view->duplicate_layer(0), 1);
    f.view->set_active_layer(QStringLiteral("1"));
    frame.setActiveTool(pictura::ToolId::Move);
    QVERIFY2(f.view->move_preview_base().isNull(), "switching to the tool builds nothing");

    // At 1/8 zoom the canvas presents level 3; a paint reports it.
    f.canvas->setZoom(0.125, QPointF(f.canvas->width() / 2.0, f.canvas->height() / 2.0));
    f.canvas->repaint();
    f.canvas->mousePressed(QPointF(1200, 1000), Qt::LeftButton, int(Qt::NoModifier));
    QVERIFY(f.canvas->movePreviewActive());
    QCOMPARE(pictura::move_preview_scale(*f.view), 8);
    const QImage layer = f.view->move_preview_layer();
    QVERIFY2(!layer.isNull() && layer.width() <= 2400 / 8 && layer.height() <= 2000 / 8,
             "the moving image is at the presented level");
    QCOMPARE(f.canvas->moveOutlineForTest(), QRectF(0, 0, 2400, 2000));

    f.canvas->mouseMoved(QPointF(1240, 1016));
    QCOMPARE(f.canvas->moveOutlineForTest().topLeft(), QPointF(40, 16));
    f.canvas->mouseReleased(QPointF(1240, 1016));
    QVERIFY(!f.canvas->movePreviewActive());
    const pictura::transform_api::LayerRect moved =
        pictura::transform_api::layerRect(*f.view, QStringLiteral("1"));
    QVERIFY(moved.valid);
    QCOMPARE(QPoint(moved.left, moved.top), QPoint(40, 16));
}

QTEST_MAIN(MoveFeedbackTest)
#include "tst_move_feedback.moc"
