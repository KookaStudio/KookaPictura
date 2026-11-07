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
#include "api/paint_bridge.h"
#include "api/transform_bridge.h"

#include <memory>

#include "qt_test_support.h"

#include <QtGui/QPainter>

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
    void moveThenFillOutsideTheLayer();
    void movedLayerStaysUnderTheLayersAbove();
    void linkedSetStaysUnderTheLayersAbove();
    void antsRepaintClearsThePreviousFrame();

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

void MoveFeedbackTest::moveThenFillOutsideTheLayer()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_move_grow"));
    QVERIFY2(f.ok(), "move fixture");
    showFrame();
    f.canvas->setSmartGuidesEnabled(false);

    // A small mark on a new layer, then the Move tool drags it.
    const int layer = f.view->add_layer(0);
    QVERIFY(layer >= 0);
    f.view->set_active_layer(QString::number(layer));
    QVERIFY(f.view->select_rect(40, 40, 20, 20, QStringLiteral("new"), 0.0));
    const QColor foreground(10, 20, 30);
    QVERIFY(pictura::edit_fill(*f.view, foreground.rgba(), -1, QStringLiteral("normal"), 100,
                               false));
    f.view->deselect();
    frame.setActiveTool(pictura::ToolId::Move);

    f.canvas->mousePressed(QPointF(50, 50), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(80, 50));
    f.canvas->mouseReleased(QPointF(80, 50));
    QCOMPARE(f.view->sample_argb(85, 50), foreground.rgba());
    QCOMPARE(f.view->sample_argb(45, 50), QColor(Qt::white).rgba());

    // The moved layer is content-sized; a fill outside it grows it (#1208).
    QVERIFY(f.view->select_rect(5, 5, 10, 10, QStringLiteral("new"), 0.0));
    const int base = f.view->history_index();
    QVERIFY(pictura::edit_fill(*f.view, foreground.rgba(), -1, QStringLiteral("normal"), 100,
                               false));
    QVERIFY2(f.committedOnce(base, "Fill"), "the moved layer grows for a fill");
    QCOMPARE(f.view->sample_argb(10, 10), foreground.rgba());
}

// [lct_move_preview_stack] a moved layer keeps the layers above it on top
void MoveFeedbackTest::movedLayerStaysUnderTheLayersAbove()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_move_stack"));
    QVERIFY2(f.ok(), "move fixture");
    showFrame();
    f.canvas->setSmartGuidesEnabled(false);

    // A blue square below a red square; the two overlap once the blue moves.
    const int blueIndex = f.view->add_layer(0);
    QVERIFY(blueIndex > 0);
    const QString blue = QString::number(blueIndex);
    f.view->set_active_layer(blue);
    QVERIFY(f.view->select_rect(20, 20, 40, 40, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*f.view, qRgba(0, 0, 255, 255), -1, QStringLiteral("normal"), 100,
                               false));
    f.view->deselect();
    const int redIndex = f.view->add_layer(blueIndex);
    QVERIFY(redIndex > blueIndex);
    f.view->set_active_layer(QString::number(redIndex));
    QVERIFY(f.view->select_rect(40, 40, 40, 40, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*f.view, qRgba(255, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    f.view->deselect();

    frame.setActiveTool(pictura::ToolId::Move);
    f.view->set_active_layer(blue);
    QCoreApplication::processEvents();

    // Press the blue (the red does not reach its top-left), drag it under the
    // red. The blue lands at (60, 40) - (100, 80); the red stays at (40, 40).
    f.canvas->mousePressed(QPointF(30.0, 30.0), Qt::LeftButton, int(Qt::NoModifier));
    QVERIFY(f.canvas->movePreviewActive());
    f.canvas->mouseMoved(QPointF(70.0, 50.0));
    const QImage shot = f.canvas->grab().toImage();
    const auto at = [&](double x, double y) {
        return shot.pixelColor(f.canvas->imageToWidget(QPointF(x, y)).toPoint());
    };
    QCOMPARE(at(70.0, 50.0), QColor(Qt::red));    // the layer above stays on top
    QCOMPARE(at(90.0, 45.0), QColor(Qt::blue));   // the moving content shows
    QCOMPARE(at(25.0, 25.0), QColor(Qt::white));  // the old place is the backdrop
    f.canvas->mouseReleased(QPointF(70.0, 50.0));
}

// [lct_move_preview_link_stack] a moved linked set keeps the layers above it on top
void MoveFeedbackTest::linkedSetStaysUnderTheLayersAbove()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_move_link_stack"));
    QVERIFY2(f.ok(), "move fixture");
    showFrame();
    f.canvas->setSmartGuidesEnabled(false);

    const auto square = [&](int above, const QRect& box, QRgb colour) {
        const int index = f.view->add_layer(above);
        if (index < 0) {
            return QString();
        }
        const QString path = QString::number(index);
        f.view->set_active_layer(path);
        if (!f.view->select_rect(box.x(), box.y(), box.width(), box.height(),
                                 QStringLiteral("new"), 0.0)
            || !pictura::edit_fill(*f.view, colour, -1, QStringLiteral("normal"), 100, false)) {
            return QString();
        }
        f.view->deselect();
        return path;
    };
    const QString a = square(0, QRect(10, 10, 30, 30), qRgba(0, 0, 255, 255));
    QVERIFY2(!a.isEmpty(), "first linked square");
    const QString b = square(a.toInt(), QRect(90, 10, 30, 30), qRgba(0, 0, 255, 255));
    QVERIFY2(!b.isEmpty(), "second linked square");
    const QString red = square(b.toInt(), QRect(50, 40, 40, 40), qRgba(255, 0, 0, 255));
    QVERIFY2(!red.isEmpty(), "layer above the set");
    QCOMPARE(f.view->link_layers({a, b}, true), 2);

    frame.setActiveTool(pictura::ToolId::Move);
    f.view->set_active_layer(a);
    QCoreApplication::processEvents();

    // Drag the set by (40, 30): the first square lands under the red layer.
    f.canvas->mousePressed(QPointF(20.0, 20.0), Qt::LeftButton, int(Qt::NoModifier));
    QVERIFY(f.canvas->movePreviewActive());
    f.canvas->mouseMoved(QPointF(60.0, 50.0));
    const QImage shot = f.canvas->grab().toImage();
    const auto at = [&](double x, double y) {
        return shot.pixelColor(f.canvas->imageToWidget(QPointF(x, y)).toPoint());
    };
    QCOMPARE(at(60.0, 50.0), QColor(Qt::red));    // the layer above stays on top
    QCOMPARE(at(135.0, 50.0), QColor(Qt::blue));  // the second member shows
    f.canvas->mouseReleased(QPointF(60.0, 50.0));
}

// [cvda_move_previous_frame] a repaint limited to the ants clears the last frame
void MoveFeedbackTest::antsRepaintClearsThePreviousFrame()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_move_repaint"));
    QVERIFY2(f.ok(), "move fixture");
    showFrame();
    f.canvas->setSmartGuidesEnabled(false);

    const int index = f.view->add_layer(0);
    QVERIFY(index > 0);
    f.view->set_active_layer(QString::number(index));
    QVERIFY(f.view->select_rect(20, 20, 40, 40, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*f.view, qRgba(0, 0, 255, 255), -1, QStringLiteral("normal"), 100,
                               false));
    f.view->deselect();
    frame.setActiveTool(pictura::ToolId::Move);

    f.canvas->mousePressed(QPointF(30.0, 30.0), Qt::LeftButton, int(Qt::NoModifier));
    QVERIFY(f.canvas->movePreviewActive());
    f.canvas->mouseMoved(QPointF(40.0, 30.0));
    QImage shot = f.canvas->grab().toImage();
    const QPoint previous = f.canvas->imageToWidget(QPointF(45.0, 30.0)).toPoint();
    QCOMPARE(shot.pixelColor(previous), QColor(Qt::blue));

    // Move again, then keep only what a repaint limited to the marching ants
    // would repaint: the previous frame's pixels must not survive it.
    f.canvas->mouseMoved(QPointF(60.0, 30.0));
    const QRect ants = f.canvas->antsUpdateRectForTest();
    const QImage current = f.canvas->grab().toImage();
    QPainter painter(&shot);
    painter.setClipRect(ants);
    painter.drawImage(QPoint(), current);
    painter.end();
    QCOMPARE(shot.pixelColor(previous), QColor(Qt::white));
    QCOMPARE(shot.pixelColor(f.canvas->imageToWidget(QPointF(75.0, 30.0)).toPoint()),
             QColor(Qt::blue));
    f.canvas->mouseReleased(QPointF(60.0, 30.0));
}

QTEST_MAIN(MoveFeedbackTest)
#include "tst_move_feedback.moc"
