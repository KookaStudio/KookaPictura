// The Move tool's Show Transform Controls box (#296 follow-up): the box shown
// on the active layer, its per-drag handle gestures, and the Move drags that
// pass through it.

#include <QtTest/QtTest>

#include "frame.h"
#include "image_view.h"
#include "options_bar.h"
#include "tools.h"

#include "commands.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_transform/bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include "qt_test_support.h"
#include "selftest_paint_fixture.h"

#include <QtWidgets/QCheckBox>
#include <QtCore/QCoreApplication>
#include <QtGui/QAction>

namespace {

// The idle controls box on the canvas as "left top right bottom", or empty.
QString boxRect(const pictura::ImageView& canvas)
{
    const QPolygonF box = canvas.controlsBoxForTest();
    if (box.size() < 4) {
        return QString();
    }
    const QRectF r = box.boundingRect();
    return QStringLiteral("%1 %2 %3 %4").arg(r.left()).arg(r.top()).arg(r.right()).arg(r.bottom());
}

} // namespace

class TransformControlsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void checkingTheBoxShowsControls();
    void boxFollowsTheActiveLayer();
    void handleDragScalesAndCommits();
    void handlePressPatchesTheBackdrop();
    void previewsKeepTheCanvasPixels();
    void insideDragMovesAndRecordsMove();
    void clickInsideLeavesTheDocumentAlone();
    void selectionSuppressesTheBox();
    void backgroundAndToolSwitchSuppressTheBox();
    void hiddenLayerShowsNoControlsOrMove();
    void hiddenLayerRefusesTransforms();
    void ctrlTTakesTheBoxOver();
    void idleBoxLeavesTheLayerAlone();

private:
    // New document + one filled raster layer selected, Move active; toggles the
    // Show Transform Controls checkbox. Returns the layer's path.
    QString beginControls(pictura::PicturaMainWindow& frame, bool checkBox);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void TransformControlsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void TransformControlsTest::cleanup()
{
    if (window_) {
        // The controller keeps the option across documents; start each test off.
        if (auto* box = window_->findChild<QCheckBox*>(QStringLiteral("optionsMoveTransform"))) {
            box->setChecked(false);
        }
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
}

QString TransformControlsTest::beginControls(pictura::PicturaMainWindow& frame, bool checkBox)
{
    if (!frame.newDocument(QStringLiteral("Controls"), 40, 30, QStringLiteral("rgb"), 8,
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
    if (!view->select_rect(10, 8, 20, 20, QStringLiteral("new"), 0.0)
        || !pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false)) {
        return QString();
    }
    view->deselect();
    frame.setActiveTool(pictura::ToolId::Move);
    auto* box = frame.findChild<QCheckBox*>(QStringLiteral("optionsMoveTransform"));
    if (!box) {
        return QString();
    }
    box->setChecked(checkBox);
    return layer;
}

void TransformControlsTest::checkingTheBoxShowsControls()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* bar = frame.findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(view && canvas && tools && bar);

    // The box is up but idle: drawn, no session until a handle is grabbed.
    QVERIFY(tools->transformControlsSession());
    QVERIFY(tools->transformControlsIdleForTest());
    QVERIFY(!view->transform_session_active());
    // CS6's controls box has no reference point.
    QVERIFY(!canvas->transformPivotVisibleForTest());
    // The bar stays on the Move page.
    QCOMPARE(bar->currentPageForTest(), bar->pageForTest(pictura::ToolId::Move));
    // The box hugs the filled square.
    QCOMPARE(boxRect(*canvas), QStringLiteral("10 8 30 28"));

    // Unchecking hides the box without recording anything.
    const int history = view->history_count();
    auto* box = frame.findChild<QCheckBox*>(QStringLiteral("optionsMoveTransform"));
    QVERIFY(box);
    box->setChecked(false);
    QVERIFY(!tools->transformControlsSession());
    QVERIFY(boxRect(*canvas).isEmpty());
    QCOMPARE(view->history_count(), history);
}

void TransformControlsTest::boxFollowsTheActiveLayer()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString first = beginControls(frame, true);
    QVERIFY(!first.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(view && canvas);
    QCOMPARE(boxRect(*canvas), QStringLiteral("10 8 30 28"));

    const int added = view->add_layer(0);
    QVERIFY(added > 0);
    const QString second = QString::number(added);
    frame.selectLayerPath(second);
    QVERIFY(view->select_rect(2, 2, 6, 6, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, qRgba(255, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    view->deselect();

    // The box rebuilt on the new layer's content rect.
    QCOMPARE(boxRect(*canvas), QStringLiteral("2 2 8 8"));
}

void TransformControlsTest::handleDragScalesAndCommits()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && canvas && tools);

    const int base = view->history_count();
    canvas->mousePressed(QPointF(10.0, 8.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(QPointF(4.0, 2.0));
    canvas->mouseReleased(QPointF(4.0, 2.0));

    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Free Transform"));
    // The box returns, idle, on the scaled layer.
    QVERIFY(tools->transformControlsIdleForTest());
    QVERIFY(!view->transform_session_active());
    QCOMPARE(boxRect(*canvas), QStringLiteral("4 2 30 28"));
}

// [tpr_canvas_patch]
void TransformControlsTest::handlePressPatchesTheBackdrop()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(view && canvas);
    view->set_layer_visible(0, false);
    canvas->setSmartGuidesEnabled(false);
    const QColor checker = canvas->grab().toImage().pixelColor(
        canvas->imageToWidget(QPointF(35.0, 25.0)).toPoint());

    // Shrink the layer toward its top-left corner, off its bottom-right quarter.
    canvas->mousePressed(QPointF(30.0, 28.0), Qt::LeftButton, int(Qt::NoModifier));
    QCOMPARE(view->move_preview_base().size(), QSize(20, 20));
    const auto origin = pictura::move_preview_base_origin(*view);
    QCOMPARE(int(origin.size()), 2);
    QCOMPARE(QPoint(origin[0], origin[1]), QPoint(10, 8));
    canvas->mouseMoved(QPointF(20.0, 18.0));

    const QImage shot = canvas->grab().toImage();
    const auto at = [&](double x, double y) {
        return shot.pixelColor(canvas->imageToWidget(QPointF(x, y)).toPoint());
    };
    QCOMPARE(at(13.0, 11.0), QColor(Qt::black));
    QVERIFY2(at(27.0, 25.0) != QColor(Qt::black), "the layer still shows at its old place");
    QVERIFY2(at(27.0, 25.0).lightness() > 150, "the old place is not the checkerboard");
    QCOMPARE(at(35.0, 25.0), checker);
    canvas->mouseReleased(QPointF(20.0, 18.0));
}

// [tpr_canvas_kept]
void TransformControlsTest::previewsKeepTheCanvasPixels()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(400, 300, QImage::Format_RGB32);
    for (int y = 0; y < seed.height(); ++y) {
        for (int x = 0; x < seed.width(); ++x) {
            const int v = (x * 37 + y * 91) % 256;
            seed.setPixel(x, y, qRgb(v, 255 - v, (x ^ y) & 0xff));
        }
    }
    paint_fixture::Fixture f(frame, seed, QStringLiteral("pictura_preview_canvas_seed"));
    QVERIFY(f.ok());
    const int added = f.view->add_layer(0);
    QVERIFY(added > 0);
    frame.selectLayerPath(QString::number(added));
    QVERIFY(f.view->select_rect(100, 100, 40, 40, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*f.view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    f.view->deselect();
    frame.setActiveTool(pictura::ToolId::Move);
    auto* box = frame.findChild<QCheckBox*>(QStringLiteral("optionsMoveTransform"));
    QVERIFY(box);
    box->setChecked(true);
    f.canvas->setSmartGuidesEnabled(false);
    f.canvas->setZoom(0.58, QPointF(200.0, 150.0));

    // The document's top strip: above the layer, its box, and the pointer.
    const auto strip = [&f] {
        // The first grab lays the hidden canvas out, so it comes before the mapping.
        const QImage shot = f.canvas->grab().toImage();
        const QPoint a = f.canvas->imageToWidget(QPointF(10.0, 10.0)).toPoint();
        const QPoint b = f.canvas->imageToWidget(QPointF(390.0, 70.0)).toPoint();
        return shot.copy(QRect(a, b));
    };
    const QImage before = strip();
    QVERIFY(!before.isNull());
    QVERIFY2(before.pixelColor(5, 5) != before.pixelColor(170, 25), "the strip misses the image");

    f.canvas->mousePressed(QPointF(120.0, 120.0), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(250.0, 200.0));
    QVERIFY(f.canvas->movePreviewActive());
    QCOMPARE(f.view->move_preview_base().size(), QSize(40, 40));
    QVERIFY2(strip() == before, "a Move drag changed the canvas outside the layer");
    f.canvas->mouseMoved(QPointF(120.0, 120.0));
    f.canvas->mouseReleased(QPointF(120.0, 120.0));

    f.canvas->mousePressed(QPointF(140.0, 140.0), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(125.0, 125.0));
    QVERIFY(f.canvas->transformPreviewActive());
    QVERIFY2(strip() == before, "a handle drag changed the canvas outside the layer");
    f.canvas->mouseReleased(QPointF(125.0, 125.0));
}

void TransformControlsTest::insideDragMovesAndRecordsMove()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && canvas && tools);

    const int base = view->history_count();
    // This check is about the inside drag moving the layer, not snapping.
    canvas->setSmartGuidesEnabled(false);
    canvas->mousePressed(QPointF(15.0, 13.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(QPointF(20.0, 16.0));
    canvas->mouseReleased(QPointF(20.0, 16.0));

    // An inside drag is the Move tool's, not the session's.
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Move Layer"));
    QVERIFY(tools->transformControlsIdleForTest());
    QCOMPARE(boxRect(*canvas), QStringLiteral("15 11 35 31"));
}

void TransformControlsTest::clickInsideLeavesTheDocumentAlone()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && canvas && tools);

    const int history = view->history_count();
    canvas->mousePressed(QPointF(15.0, 13.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseReleased(QPointF(15.0, 13.0));
    QCOMPARE(view->history_count(), history);
    // The box is back even though nothing changed.
    QVERIFY(tools->transformControlsIdleForTest());
    QCOMPARE(boxRect(*canvas), QStringLiteral("10 8 30 28"));
}

void TransformControlsTest::selectionSuppressesTheBox()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(view && canvas);
    QVERIFY(!boxRect(*canvas).isEmpty());

    QVERIFY(view->select_rect(5, 5, 6, 6, QStringLiteral("new"), 0.0));
    QVERIFY(boxRect(*canvas).isEmpty());

    view->deselect();
    QVERIFY(!boxRect(*canvas).isEmpty());
}

void TransformControlsTest::backgroundAndToolSwitchSuppressTheBox()
{
    pictura::PicturaMainWindow& frame = *window_;
    auto* box = frame.findChild<QCheckBox*>(QStringLiteral("optionsMoveTransform"));
    QVERIFY(box);
    // A Background document has no transformable layer: no box.
    QVERIFY(frame.newDocument(QStringLiteral("ControlsBg"), 20, 20, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    // A white new document's layer is an ordinary layer; flag it as the
    // Background so it is refused.
    QVERIFY(view->background_from_layer(QStringLiteral("0")));
    frame.setActiveTool(pictura::ToolId::Move);
    box->setChecked(true);
    QVERIFY(!view->transform_session_active());
    QVERIFY(boxRect(*frame.imageView()).isEmpty());
    frame.closeDocument(frame.activeDocumentIndex(), false);

    // A tool switch hides a shown box; returning to Move shows it again.
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools && tools->transformControlsSession());
    frame.setActiveTool(pictura::ToolId::Brush);
    QVERIFY(!tools->transformControlsSession());
    QVERIFY(boxRect(*frame.imageView()).isEmpty());
    frame.setActiveTool(pictura::ToolId::Move);
    QVERIFY(tools->transformControlsSession());
    QVERIFY(!boxRect(*frame.imageView()).isEmpty());
}

void TransformControlsTest::hiddenLayerShowsNoControlsOrMove()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && canvas && tools);
    QVERIFY(tools->transformControlsSession());
    QVERIFY(!boxRect(*canvas).isEmpty());

    view->set_layer_visible(layer.toInt(), false);
    QCoreApplication::processEvents();

    // A hidden layer shows no controls box.
    QVERIFY(!tools->transformControlsSession());
    QVERIFY(boxRect(*canvas).isEmpty());

    const auto rectOf = [view, &layer]() {
        const ::rust::Vec<int32_t> rect = pictura::layer_rect_extents(*view, layer);
        return QList<int>({rect[0], rect[1], rect[2], rect[3]});
    };
    const QList<int> before = rectOf();
    const int history = view->history_count();
    canvas->setSmartGuidesEnabled(false);
    canvas->mousePressed(QPointF(15.0, 13.0), Qt::LeftButton, int(Qt::NoModifier));
    QVERIFY(!canvas->movePreviewActive());
    canvas->mouseMoved(QPointF(25.0, 23.0));
    canvas->mouseReleased(QPointF(25.0, 23.0));

    // The Move drag neither moves the hidden layer nor records a state.
    QCOMPARE(rectOf(), before);
    QCOMPARE(view->history_count(), history);
}

void TransformControlsTest::hiddenLayerRefusesTransforms()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    pictura::CommandRegistry* registry = frame.registry();
    QVERIFY(registry);
    QAction* free = registry->action(QString::fromLatin1(pictura::command_ids::EditFreeTransform));
    QVERIFY(free);

    view->set_layer_visible(layer.toInt(), false);
    QCoreApplication::processEvents();
    registry->refresh();

    QVERIFY(!view->layer_can_free_transform(layer));
    QVERIFY(!free->isEnabled());
    QVERIFY(!view->begin_free_transform(layer));
    QVERIFY(!view->transform_session_active());
    QVERIFY(!frame.beginFreeTransform(layer));
    QVERIFY(!view->begin_move_preview());

    // A selection cannot lift a hidden layer's pixels either.
    QVERIFY(view->select_rect(5, 5, 6, 6, QStringLiteral("new"), 0.0));
    QVERIFY(!view->layer_can_free_transform(layer));
    QVERIFY(!view->begin_free_transform(layer));
    QVERIFY(!view->transform_session_active());
    view->deselect();
}

void TransformControlsTest::ctrlTTakesTheBoxOver()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* bar = frame.findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(view && tools && bar);
    QVERIFY(tools->transformControlsSession());

    QVERIFY(frame.beginFreeTransform(layer));
    QVERIFY(!tools->transformControlsSession());
    QVERIFY(view->transform_session_active());
    QCOMPARE(bar->currentPageForTest(), bar->transformPageForTest());

    // The controls box returns once the modal session ends.
    tools->cancelFreeTransform();
    QVERIFY(tools->transformControlsSession());
    QCOMPARE(bar->currentPageForTest(), bar->pageForTest(pictura::ToolId::Move));
}

void TransformControlsTest::idleBoxLeavesTheLayerAlone()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(view && canvas);
    // The box hugs the square, but the canvas-sized layer keeps its extent, so
    // a filter run now can still spread past the square.
    QCOMPARE(boxRect(*canvas), QStringLiteral("10 8 30 28"));
    const ::rust::Vec<int32_t> rect = pictura::layer_rect_extents(*view, layer);
    QCOMPARE(int(rect.size()), 4);
    QCOMPARE(QList<int>({rect[0], rect[1], rect[2], rect[3]}), QList<int>({0, 0, 40, 30}));
}

QTEST_MAIN(TransformControlsTest)
#include "tst_transform_controls.moc"
