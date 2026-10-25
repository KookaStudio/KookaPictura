// Edit > Transform > Warp (#751): the Free Transform session's Warp mode, ported
// from photocraft. The menu and the options-bar toggle, the style/Bend/H/V
// fields, anchor drags and surface pulls on the canvas, and the commit, cancel
// and identity contracts.

#include <QtTest/QtTest>

#include "api/transform_api.h"
#include "commands.h"
#include "frame.h"
#include "image_view.h"
#include "options_bar.h"
#include "panels/numeric_field.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_transform/bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_transform/warp.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include "qt_test_support.h"

#include <QtWidgets/QComboBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QToolButton>

namespace {

constexpr double kTolerance = 1e-6;
// `impl_transform/warp.rs`: a press on a control point, and on the surface.
constexpr int kWarpPoint = 11;
constexpr int kWarpSurface = 12;

QPolygonF overlayQuad(const pictura::transform_api::WarpOverlay& overlay)
{
    if (overlay.grid.size() < 4) {
        return {};
    }
    QPolygonF quad;
    quad << overlay.grid.at(0).first() << overlay.grid.at(1).first()
         << overlay.grid.at(1).last() << overlay.grid.at(0).last();
    return quad;
}

bool fillBox(pictura::PictureView* view, const QRect& box)
{
    if (!view->select_rect(box.x(), box.y(), box.width(), box.height(), QStringLiteral("new"),
                           0.0)) {
        return false;
    }
    const bool ok = pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1,
                                       QStringLiteral("normal"), 100, false);
    view->deselect();
    return ok;
}

} // namespace

class TransformWarpTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void menuOpensWarpMode();
    void toggleSwitchesBetweenBoxAndWarp();
    void boxTransformCarriesIntoTheMesh();
    void presetStyleAndValuesFromTheBar();
    void anchorDragCommitsOneState();
    void releaseHidesTheDraggedHandles();
    void reenteringWarpKeepsTheMesh();
    void leavingWarpPutsTheBoxOnThePreviewBounds();
    void cancelWarpLeavesNoOverlay();
    void undoAfterWarpCommitRestores();
    void boxDragKeepsTheWarpedPreview();
    void boxScaleCommitsWithTheWarp();
    void numericBoxEditRefreshesTheWarpedPreview();
    void exactCommandRefreshesTheWarpedPreview();
    void perspectiveBoxWarpPreviews();
    void pressOnAPresetMakesItCustom();
    void escapeRestoresAWarpedSession();
    void identityWarpCommitsNothing();

private:
    // A new document with one filled raster layer selected; returns its path.
    // The layer's content box is (10, 8)-(22, 17).
    QString beginLayer(pictura::PicturaMainWindow& frame);
    static void typeInto(pictura::NumericField* field, const QString& text);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void TransformWarpTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void TransformWarpTest::cleanup()
{
    while (window_ && window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
}

QString TransformWarpTest::beginLayer(pictura::PicturaMainWindow& frame)
{
    if (!frame.newDocument(QStringLiteral("Warp"), 40, 30, QStringLiteral("rgb"), 8,
                           QStringLiteral("white"))) {
        return QString();
    }
    pictura::PictureView* view = frame.activeView();
    if (!view) {
        return QString();
    }
    const int added = view->add_layer(0);
    if (added <= 0) {
        return QString();
    }
    const QString layer = QString::number(added);
    frame.selectLayerPath(layer);
    if (!view->select_rect(10, 8, 12, 9, QStringLiteral("new"), 0.0)
        || !pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false)) {
        return QString();
    }
    view->deselect();
    return layer;
}

void TransformWarpTest::typeInto(pictura::NumericField* field, const QString& text)
{
    auto* edit = field->findChild<QLineEdit*>();
    QVERIFY(edit);
    edit->clear();
    QTest::keyClicks(edit, text);
    QTest::keyClick(edit, Qt::Key_Return);
}

void TransformWarpTest::menuOpensWarpMode()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* canvas = frame.findChild<pictura::ImageView*>();
    QVERIFY(view && canvas);

    QVERIFY(frame.registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::EditTransformWarp)));
    QVERIFY(view->transform_session_active());
    QVERIFY(pictura::transform_warp_active(*view));
    QCOMPARE(pictura::transform_warp_style_id(*view), QStringLiteral("custom"));

    // The bar swaps the box fields for the warp fields.
    auto* warpToggle = frame.findChild<QToolButton*>(QStringLiteral("optionsTransformWarp"));
    auto* fields = frame.findChild<QWidget*>(QStringLiteral("optionsWarpFields"));
    auto* angle = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsTransformAngle"));
    QVERIFY(warpToggle && fields && angle);
    QVERIFY(warpToggle->isChecked());
    QVERIFY(!fields->isHidden());
    QVERIFY(angle->isHidden());

    // The canvas draws the one-patch grid: four boundaries, four thirds, and
    // the twelve anchors and handles on its section lines.
    const pictura::transform_api::WarpOverlay& overlay = canvas->transformWarpForTest();
    QCOMPARE(overlay.grid.size(), 4);
    QCOMPARE(overlay.guides.size(), 4);
    QCOMPARE(overlay.points.size(), 12);
    int anchors = 0;
    for (const auto& point : overlay.points) {
        anchors += point.kind == pictura::transform_api::WarpPoint::Anchor ? 1 : 0;
    }
    QCOMPARE(anchors, 4);
    QCOMPARE(overlay.points.front().pos, QPointF(10.0, 8.0));

    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(tools);
    tools->cancelFreeTransform();
}

void TransformWarpTest::toggleSwitchesBetweenBoxAndWarp()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* canvas = frame.findChild<pictura::ImageView*>();
    auto* warpToggle = frame.findChild<QToolButton*>(QStringLiteral("optionsTransformWarp"));
    auto* angle = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsTransformAngle"));
    QVERIFY(view && tools && canvas && warpToggle && angle);
    QVERIFY(tools->beginFreeTransform(layer));

    warpToggle->click();
    QVERIFY(pictura::transform_warp_active(*view));
    QVERIFY(angle->isHidden());
    // Box gestures give way to the grid: the top-left corner is an anchor now.
    QCOMPARE(view->transform_hit_test(10.0, 8.0, 1.0), kWarpPoint);

    warpToggle->click();
    QVERIFY(!pictura::transform_warp_active(*view));
    QVERIFY(pictura::transform_warp_stored(*view));
    QCOMPARE(pictura::transform_warp_style_id(*view), QStringLiteral("custom"));
    QVERIFY(!angle->isHidden());
    QVERIFY(canvas->transformWarpForTest().isEmpty());
    QCOMPARE(view->transform_hit_test(10.0, 8.0, 1.0), 0);
    tools->cancelFreeTransform();
}

void TransformWarpTest::boxTransformCarriesIntoTheMesh()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* canvas = frame.findChild<pictura::ImageView*>();
    QVERIFY(view && tools && canvas && tools->beginFreeTransform(layer));

    QVERIFY(tools->setTransformAngle(30.0));
    const QPolygonF box = pictura::transform_api::quad(*view).corners;
    QVERIFY(tools->enterTransformWarp());
    const QPolygonF grid = overlayQuad(canvas->transformWarpForTest());
    QCOMPARE(grid.size(), 4);
    for (int i = 0; i < 4; ++i) {
        QVERIFY(QLineF(box.at(i), grid.at(i)).length() < kTolerance);
    }
    // Leaving Warp gives the turned box back with the warp still applied.
    QVERIFY(tools->leaveTransformWarp());
    QVERIFY(pictura::transform_warp_stored(*view));
    QCOMPARE(pictura::transform_warp_style_id(*view), QStringLiteral("custom"));
    QVERIFY(qAbs(pictura::transform_angle_degrees(*view) - 30.0) < kTolerance);
    tools->cancelFreeTransform();
}

void TransformWarpTest::presetStyleAndValuesFromTheBar()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* canvas = frame.findChild<pictura::ImageView*>();
    QVERIFY(view && tools && canvas && tools->beginWarp(layer));

    auto* style = frame.findChild<QComboBox*>(QStringLiteral("optionsWarpStyle"));
    auto* bend = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsWarpBend"));
    auto* h = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsWarpH"));
    auto* orientation = frame.findChild<QToolButton*>(QStringLiteral("optionsWarpOrientation"));
    QVERIFY(style && bend && h && orientation);
    QVERIFY(!bend->isEnabled());

    const QImage plain = view->move_preview_layer();
    style->setCurrentIndex(style->findData(QStringLiteral("arc")));
    QCOMPARE(pictura::transform_warp_style_id(*view), QStringLiteral("arc"));
    QCOMPARE(pictura::transform_warp_bend(*view), 50.0);
    QVERIFY(bend->isEnabled());
    QCOMPARE(bend->value(), 50.0);
    // A preset has no points to drag, and the preview shows the arc.
    QVERIFY(canvas->transformWarpForTest().points.isEmpty());
    QVERIFY(view->move_preview_layer().height() > plain.height());

    typeInto(bend, QStringLiteral("-25"));
    typeInto(h, QStringLiteral("10"));
    QCOMPARE(pictura::transform_warp_bend(*view), -25.0);
    QCOMPARE(pictura::transform_warp_h_distort(*view), 10.0);
    orientation->click();
    QVERIFY(pictura::transform_warp_vertical(*view));

    // None resets to the untouched one-patch grid.
    style->setCurrentIndex(style->findData(QStringLiteral("none")));
    QCOMPARE(pictura::transform_warp_style_id(*view), QStringLiteral("custom"));
    QVERIFY(!bend->isEnabled());
    tools->cancelFreeTransform();
}

void TransformWarpTest::anchorDragCommitsOneState()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* canvas = frame.findChild<pictura::ImageView*>();
    QVERIFY(view && tools && canvas && tools->beginWarp(layer));

    const QString before = pictura::transform_api::layerRect(*view, layer).toString();
    const int base = view->history_count();
    // Drag the top-left anchor up and left on the canvas.
    canvas->mousePressed(QPointF(10.0, 8.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(QPointF(4.0, 2.0));
    canvas->mouseReleased(QPointF(4.0, 2.0));
    QCOMPARE(canvas->transformWarpForTest().points.front().pos, QPointF(4.0, 2.0));
    QCOMPARE(view->history_count(), base);

    auto* apply = frame.findChild<QToolButton*>(QStringLiteral("optionsTransformApply"));
    QVERIFY(apply);
    apply->click();
    QVERIFY(!view->transform_session_active());
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Free Transform"));
    const pictura::transform_api::LayerRect after = pictura::transform_api::layerRect(*view, layer);
    QVERIFY(after.toString() != before);
    QCOMPARE(after.left, 4);
    QCOMPARE(after.top, 2);
}

void TransformWarpTest::releaseHidesTheDraggedHandles()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* canvas = frame.findChild<pictura::ImageView*>();
    QVERIFY(view && tools && canvas && tools->beginWarp(layer));

    const auto draggedHandles = [canvas]() {
        int n = 0;
        for (const pictura::transform_api::WarpPoint& p : canvas->transformWarpForTest().points) {
            n += p.kind == pictura::transform_api::WarpPoint::DraggedHandle ? 1 : 0;
        }
        return n;
    };
    canvas->mousePressed(QPointF(10.0, 8.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(QPointF(6.0, 5.0));
    QCOMPARE(draggedHandles(), 2);
    // The release hands the canvas a fresh overlay: the anchor's handles are
    // ordinary again, shown only near the pointer.
    canvas->mouseReleased(QPointF(6.0, 5.0));
    QCOMPARE(draggedHandles(), 0);
    tools->cancelFreeTransform();
}

void TransformWarpTest::pressOnAPresetMakesItCustom()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools && tools->beginWarp(layer));
    QVERIFY(tools->setTransformWarpStyle(QStringLiteral("bulge")));

    // Nothing to grab far outside the grid: the preset stays.
    QCOMPARE(view->transform_press(39.0, 29.0, 1.0, false, false), -1);
    QCOMPARE(pictura::transform_warp_style_id(*view), QStringLiteral("bulge"));
    // A press inside (zoomed in, clear of the handles) pulls the surface and
    // turns the preset into a mesh.
    QCOMPARE(view->transform_press(16.0, 12.5, 8.0, false, false), kWarpSurface);
    QCOMPARE(pictura::transform_warp_style_id(*view), QStringLiteral("custom"));
    QVERIFY(view->transform_move(18.0, 14.0, 8.0, false, false));
    QVERIFY(view->transform_release());
    tools->cancelFreeTransform();
}

void TransformWarpTest::escapeRestoresAWarpedSession()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* canvas = frame.findChild<pictura::ImageView*>();
    QVERIFY(view && tools && canvas);

    const QString before = pictura::transform_api::layerRect(*view, layer).toString();
    const int base = view->history_count();
    QVERIFY(tools->beginWarp(layer));
    QVERIFY(tools->setTransformWarpStyle(QStringLiteral("wave")));
    QTest::keyClick(canvas, Qt::Key_Escape);
    QVERIFY(!view->transform_session_active());
    QVERIFY(canvas->transformPreviewLayerForTest().isNull());
    QVERIFY(canvas->transformWarpForTest().isEmpty());
    QCOMPARE(view->history_count(), base);
    QCOMPARE(pictura::transform_api::layerRect(*view, layer).toString(), before);
}

void TransformWarpTest::identityWarpCommitsNothing()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools && tools->beginWarp(layer));

    const int base = view->history_count();
    tools->commitFreeTransform();
    QVERIFY(!view->transform_session_active());
    QCOMPARE(view->history_count(), base);
}

void TransformWarpTest::reenteringWarpKeepsTheMesh()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* canvas = frame.findChild<pictura::ImageView*>();
    QVERIFY(view && tools && canvas);
    QVERIFY(tools->beginWarp(layer));

    canvas->mousePressed(QPointF(10.0, 8.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(QPointF(4.0, 2.0));
    canvas->mouseReleased(QPointF(4.0, 2.0));
    QCOMPARE(canvas->transformWarpForTest().points.front().pos, QPointF(4.0, 2.0));

    QVERIFY(tools->leaveTransformWarp());
    QVERIFY(!pictura::transform_warp_active(*view));
    QVERIFY(pictura::transform_warp_stored(*view));
    QVERIFY(tools->enterTransformWarp());
    QVERIFY(pictura::transform_warp_active(*view));
    QCOMPARE(canvas->transformWarpForTest().points.front().pos, QPointF(4.0, 2.0));
    QCOMPARE(pictura::transform_warp_style_id(*view), QStringLiteral("custom"));
    tools->cancelFreeTransform();
}

void TransformWarpTest::leavingWarpPutsTheBoxOnThePreviewBounds()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* canvas = frame.findChild<pictura::ImageView*>();
    QVERIFY(view && tools && canvas);
    QVERIFY(tools->beginWarp(layer));

    // A custom corner drag makes the preview's bounds differ from the box.
    canvas->mousePressed(QPointF(10.0, 8.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(QPointF(4.0, 2.0));
    canvas->mouseReleased(QPointF(4.0, 2.0));
    const QImage preview = view->move_preview_layer();
    const QPointF origin(view->move_preview_x(), view->move_preview_y());
    const QSize size = preview.size();
    QVERIFY(!preview.isNull());

    QVERIFY(tools->leaveTransformWarp());
    QCOMPARE(view->move_preview_layer(), preview);
    const QPolygonF box = pictura::transform_api::quad(*view).corners;
    QCOMPARE(box.size(), 4);
    const QPointF want[4] = {
        origin,
        origin + QPointF(size.width(), 0.0),
        origin + QPointF(size.width(), size.height()),
        origin + QPointF(0.0, size.height()),
    };
    for (int i = 0; i < 4; ++i) {
        QVERIFY2(QLineF(box.at(i), want[i]).length() < kTolerance,
                 qPrintable(QStringLiteral("box corner %1").arg(i)));
    }

    // Re-entering Warp reuses the mesh, whose top-left anchor is on the box.
    QVERIFY(tools->enterTransformWarp());
    const pictura::transform_api::WarpOverlay& overlay = canvas->transformWarpForTest();
    QCOMPARE(overlay.points.size(), 12);
    QVERIFY(QLineF(overlay.points.front().pos, QPointF(4.0, 2.0)).length() < kTolerance);
    QCOMPARE(view->move_preview_layer(), preview);

    // The post-frame's inverse keeps a re-entered anchor drag on the pointer.
    canvas->mousePressed(QPointF(4.0, 2.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(QPointF(2.0, 1.0));
    canvas->mouseReleased(QPointF(2.0, 1.0));
    QVERIFY(QLineF(canvas->transformWarpForTest().points.front().pos, QPointF(2.0, 1.0))
                .length()
            < kTolerance);
    tools->cancelFreeTransform();
}

void TransformWarpTest::cancelWarpLeavesNoOverlay()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* canvas = frame.findChild<pictura::ImageView*>();
    QVERIFY(view && tools && canvas);
    const QString before = pictura::transform_api::layerRect(*view, layer).toString();
    const int base = view->history_count();

    QVERIFY(tools->beginWarp(layer));
    QVERIFY(tools->setTransformWarpStyle(QStringLiteral("arc")));
    QVERIFY(!canvas->transformPreviewLayerForTest().isNull());
    tools->cancelFreeTransform();

    QVERIFY(!view->transform_session_active());
    QVERIFY(canvas->transformPreviewLayerForTest().isNull());
    QVERIFY(pictura::transform_api::quad(*view).corners.isEmpty());
    QCOMPARE(view->history_count(), base);
    QCOMPARE(pictura::transform_api::layerRect(*view, layer).toString(), before);
}

void TransformWarpTest::undoAfterWarpCommitRestores()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* canvas = frame.findChild<pictura::ImageView*>();
    QVERIFY(view && tools && canvas);
    const QString before = pictura::transform_api::layerRect(*view, layer).toString();
    const int base = view->history_count();

    QVERIFY(tools->beginWarp(layer));
    QVERIFY(tools->setTransformWarpStyle(QStringLiteral("arc")));
    QVERIFY(tools->leaveTransformWarp());
    QVERIFY(tools->setTransformWidth(150.0, true));
    tools->commitFreeTransform();
    QVERIFY(!view->transform_session_active());
    QCOMPARE(view->history_count(), base + 1);
    QVERIFY(view->undo());
    QCOMPARE(pictura::transform_api::layerRect(*view, layer).toString(), before);
    QVERIFY(canvas->transformPreviewLayerForTest().isNull());
    QVERIFY(view->redo());
    QVERIFY(pictura::transform_api::layerRect(*view, layer).toString() != before);
}

void TransformWarpTest::boxDragKeepsTheWarpedPreview()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* canvas = frame.findChild<pictura::ImageView*>();
    QVERIFY(view && tools && canvas);
    QVERIFY(fillBox(view, QRect(5, 5, 30, 20)));
    QVERIFY(tools->beginWarp(layer));
    QVERIFY(tools->setTransformWarpStyle(QStringLiteral("arc")));
    const QSize arc = view->move_preview_layer().size();
    QCOMPARE(canvas->transformPreviewLayerForTest().size(), arc);

    QVERIFY(tools->leaveTransformWarp());
    QVERIFY(!pictura::transform_warp_active(*view));
    pictura::transform_set_show_pivot(*view, false);
    const QPolygonF box = pictura::transform_api::quad(*view).corners;
    const QPointF rightMid((box.at(1).x() + box.at(2).x()) / 2.0,
                           (box.at(1).y() + box.at(2).y()) / 2.0);
    canvas->mousePressed(rightMid, Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(rightMid + QPointF(4.0, 0.0));
    canvas->mouseReleased(rightMid + QPointF(4.0, 0.0));

    QVERIFY(view->move_preview_layer().width() > arc.width());
    QVERIFY(canvas->transformPreviewLayerForTest().width() > arc.width());
    QVERIFY(canvas->transformWarpForTest().isEmpty());
    tools->cancelFreeTransform();
}

void TransformWarpTest::boxScaleCommitsWithTheWarp()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* canvas = frame.findChild<pictura::ImageView*>();
    QVERIFY(view && tools && canvas);
    QVERIFY(fillBox(view, QRect(5, 5, 30, 20)));
    QVERIFY(tools->beginWarp(layer));
    QVERIFY(tools->setTransformWarpStyle(QStringLiteral("arc")));
    const QSize arc = view->move_preview_layer().size();
    QVERIFY(arc.height() > 20);
    QVERIFY(tools->leaveTransformWarp());
    pictura::transform_set_show_pivot(*view, false);

    const QPolygonF box = pictura::transform_api::quad(*view).corners;
    QCOMPARE(box.size(), 4);
    const QPointF origin(view->move_preview_x(), view->move_preview_y());
    QVERIFY(QLineF(box.at(0), origin).length() < kTolerance);
    QVERIFY(QLineF(box.at(2), origin + QPointF(arc.width(), arc.height())).length() < kTolerance);
    const QPointF rightMid((box.at(1).x() + box.at(2).x()) / 2.0,
                           (box.at(1).y() + box.at(2).y()) / 2.0);
    canvas->mousePressed(rightMid, Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(rightMid + QPointF(4.0, 0.0));
    canvas->mouseReleased(rightMid + QPointF(4.0, 0.0));

    const int base = view->history_count();
    QVERIFY(view->commit_transform());
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Free Transform"));
    const pictura::transform_api::LayerRect rect =
        pictura::transform_api::layerRect(*view, layer);
    QVERIFY(rect.valid);
    QVERIFY(rect.width() > 30);
    QVERIFY2(qAbs(rect.height() - arc.height()) <= 1,
             "the arc still bends the scaled layer");
}

void TransformWarpTest::numericBoxEditRefreshesTheWarpedPreview()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* canvas = frame.findChild<pictura::ImageView*>();
    QVERIFY(view && tools && canvas);
    QVERIFY(tools->beginWarp(layer));
    QVERIFY(tools->setTransformWarpStyle(QStringLiteral("arc")));
    const QSize arc = view->move_preview_layer().size();
    QCOMPARE(canvas->transformPreviewLayerForTest().size(), arc);

    QVERIFY(tools->leaveTransformWarp());
    QVERIFY(tools->setTransformWidth(200.0, true));
    QVERIFY(view->move_preview_layer().width() > arc.width());
    QVERIFY(canvas->transformPreviewLayerForTest().width() > arc.width());
    QVERIFY(!pictura::transform_warp_active(*view));
    tools->cancelFreeTransform();
}

void TransformWarpTest::exactCommandRefreshesTheWarpedPreview()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* canvas = frame.findChild<pictura::ImageView*>();
    QVERIFY(view && tools && canvas);
    QVERIFY(tools->beginWarp(layer));
    QVERIFY(tools->setTransformWarpStyle(QStringLiteral("arc")));
    const QSize arc = view->move_preview_layer().size();
    QCOMPARE(canvas->transformPreviewLayerForTest().size(), arc);

    QVERIFY(tools->leaveTransformWarp());
    QVERIFY(frame.registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::EditTransformRotate90Cw)));
    const QSize after = view->move_preview_layer().size();
    QVERIFY(qAbs(after.width() - arc.height()) <= 1);
    QVERIFY(qAbs(after.height() - arc.width()) <= 1);
    const QSize canvasAfter = canvas->transformPreviewLayerForTest().size();
    QVERIFY(qAbs(canvasAfter.width() - arc.height()) <= 1);
    QVERIFY(qAbs(canvasAfter.height() - arc.width()) <= 1);
    QVERIFY(!pictura::transform_warp_active(*view));
    tools->cancelFreeTransform();
}

void TransformWarpTest::perspectiveBoxWarpPreviews()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* canvas = frame.findChild<pictura::ImageView*>();
    QVERIFY(view && tools && canvas);
    QVERIFY(tools->beginFreeTransform(layer));

    const QPolygonF before = pictura::transform_api::quad(*view).corners;
    const QPointF corner = before.at(1);
    const int mods = int(Qt::ControlModifier | Qt::AltModifier | Qt::ShiftModifier);
    canvas->mousePressed(corner, Qt::LeftButton, mods);
    canvas->mouseMoved(corner + QPointF(10.0, -2.0));
    canvas->mouseReleased(corner + QPointF(10.0, -2.0));
    const QPolygonF box = pictura::transform_api::quad(*view).corners;

    QVERIFY(tools->enterTransformWarp());
    const QPolygonF grid = overlayQuad(canvas->transformWarpForTest());
    QCOMPARE(grid.size(), 4);
    for (int i = 0; i < 4; ++i) {
        QVERIFY(QLineF(grid.at(i), box.at(i)).length() < 1e-6);
    }
    QVERIFY(!view->move_preview_layer().isNull());
    tools->cancelFreeTransform();
}

QTEST_MAIN(TransformWarpTest)
#include "tst_transform_warp.moc"
