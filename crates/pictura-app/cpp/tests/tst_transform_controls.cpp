// The Move tool's Show Transform Controls box (#296 follow-up): the box shown
// on the active layer, its per-drag handle gestures, and the Move drags that
// pass through it.

#include <QtTest/QtTest>

#include "frame.h"
#include "image_view.h"
#include "options_bar.h"
#include "tools.h"

#include "commands.h"

#include "api/document_bridge.h"
#include "api/paint_bridge.h"
#include "api/transform_api.h"
#include "api/transform_bridge.h"

#include "qt_test_support.h"
#include "selftest_paint_fixture.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QToolButton>
#include <QtCore/QCoreApplication>
#include <QtCore/QTemporaryDir>
#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtGui/QImage>

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
    void handleDragStaysPendingThenCommits();
    void rejectCancelsThePendingTransform();
    void identityHandleDragReturnsToIdle();
    void pendingTransformRefinesTheSameSession();
    void insideDragMovesWithinThePendingTransform();
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
    void lowerLayerStaysUnderTheLayersAbove();
    void topmostLayerBuildsNoAboveImage();
    void previewEdgeMatchesTheCommittedResample();
    void everyTransformEndingClearsTheSamePreviewFields();
    void controlsSessionFlagIsDerived();
    void directTransformLifecycleOwnsTheControlsFlag();
    void movePreviewKeepsAPromotedTransform();

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
        // A handle gesture now leaves a pending transform on release.
        if (auto* tools = window_->findChild<pictura::ToolController*>()) {
            tools->cancelFreeTransform();
        }
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
    if (pictura::ImageView* canvas = frame.imageView()) {
        canvas->setPixelGridVisible(false);
        canvas->setLayerEdgesVisible(false);
        canvas->setSlicesVisible(false);
    }
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

void TransformControlsTest::handleDragStaysPendingThenCommits()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* bar = frame.findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(view && canvas && tools && bar);

    const int base = view->history_count();
    canvas->mousePressed(QPointF(10.0, 8.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(QPointF(4.0, 2.0));
    canvas->mouseReleased(QPointF(4.0, 2.0));

    // The released gesture is pending, not committed: the transform page shows
    // with its Confirm/Reject controls and the overlay has the pivot.
    QCOMPARE(view->history_count(), base);
    QVERIFY(view->transform_session_active());
    QVERIFY(tools->transformPending());
    QVERIFY(!tools->transformControlsSession());
    QVERIFY(!tools->transformControlsIdleForTest());
    QCOMPARE(bar->currentPageForTest(), bar->transformPageForTest());
    QVERIFY(canvas->transformPivotVisibleForTest());
    QVERIFY(canvas->transformPreviewActive());
    QVERIFY(boxRect(*canvas).isEmpty());

    // Apply commits the pending gesture as one Free Transform state and brings
    // the idle controls box back on the resized layer.
    auto* apply = bar->findChild<QToolButton*>(QStringLiteral("optionsTransformApply"));
    QVERIFY(apply);
    apply->click();
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Free Transform"));
    QVERIFY(!view->transform_session_active());
    QVERIFY(tools->transformControlsIdleForTest());
    QCOMPARE(bar->currentPageForTest(), bar->pageForTest(pictura::ToolId::Move));
    QCOMPARE(boxRect(*canvas), QStringLiteral("4 2 30 28"));
}

void TransformControlsTest::rejectCancelsThePendingTransform()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* bar = frame.findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(view && canvas && tools && bar);

    const int base = view->history_count();
    canvas->mousePressed(QPointF(10.0, 8.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(QPointF(4.0, 2.0));
    canvas->mouseReleased(QPointF(4.0, 2.0));
    QVERIFY(tools->transformPending());

    auto* cancel = bar->findChild<QToolButton*>(QStringLiteral("optionsTransformCancel"));
    QVERIFY(cancel);
    cancel->click();
    QCOMPARE(view->history_count(), base);
    QVERIFY(!view->transform_session_active());
    QVERIFY(tools->transformControlsIdleForTest());
    QCOMPARE(bar->currentPageForTest(), bar->pageForTest(pictura::ToolId::Move));
    QCOMPARE(boxRect(*canvas), QStringLiteral("10 8 30 28"));
}

void TransformControlsTest::identityHandleDragReturnsToIdle()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* bar = frame.findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(view && canvas && tools && bar);

    const int base = view->history_count();
    canvas->mousePressed(QPointF(10.0, 8.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseReleased(QPointF(10.0, 8.0));

    // A click on a handle leaves no session and no state.
    QCOMPARE(view->history_count(), base);
    QVERIFY(!view->transform_session_active());
    QVERIFY(!tools->transformPending());
    QVERIFY(tools->transformControlsIdleForTest());
    QCOMPARE(bar->currentPageForTest(), bar->pageForTest(pictura::ToolId::Move));
    QCOMPARE(boxRect(*canvas), QStringLiteral("10 8 30 28"));
}

void TransformControlsTest::pendingTransformRefinesTheSameSession()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* bar = frame.findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(view && canvas && tools && bar);

    const int base = view->history_count();
    canvas->mousePressed(QPointF(10.0, 8.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(QPointF(4.0, 2.0));
    canvas->mouseReleased(QPointF(4.0, 2.0));
    QVERIFY(tools->transformPending());

    // A second handle drag refines the same pending session. Hide the pivot so
    // a low zoom cannot turn the press into a pivot drag.
    tools->setTransformShowPivot(false);
    canvas->mousePressed(QPointF(30.0, 28.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(QPointF(36.0, 34.0));
    canvas->mouseReleased(QPointF(36.0, 34.0));
    QVERIFY(tools->transformPending());
    QCOMPARE(view->history_count(), base);

    auto* apply = bar->findChild<QToolButton*>(QStringLiteral("optionsTransformApply"));
    QVERIFY(apply);
    apply->click();
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Free Transform"));
    QCOMPARE(boxRect(*canvas), QStringLiteral("4 2 36 34"));
}

void TransformControlsTest::insideDragMovesWithinThePendingTransform()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* bar = frame.findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(view && canvas && tools && bar);

    const int base = view->history_count();
    canvas->mousePressed(QPointF(10.0, 8.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(QPointF(4.0, 2.0));
    canvas->mouseReleased(QPointF(4.0, 2.0));
    QVERIFY(tools->transformPending());

    // An inside press moves the target within the same pending transform, not
    // as a separate Move Layer state. Hide the pivot so it cannot claim the
    // press at a low zoom.
    tools->setTransformShowPivot(false);
    canvas->mousePressed(QPointF(12.0, 20.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(QPointF(17.0, 23.0));
    canvas->mouseReleased(QPointF(17.0, 23.0));
    QVERIFY(tools->transformPending());
    QCOMPARE(view->history_count(), base);

    auto* apply = bar->findChild<QToolButton*>(QStringLiteral("optionsTransformApply"));
    QVERIFY(apply);
    apply->click();
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Free Transform"));
    QCOMPARE(boxRect(*canvas), QStringLiteral("9 5 35 31"));
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
    // The released gesture is pending; drop it before the fixture closes.
    f.tools->cancelFreeTransform();
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
    // The box hugs the square, and the layer is trimmed to the same content
    // box (#1208); an idle box changes no pixels.
    QCOMPARE(boxRect(*canvas), QStringLiteral("10 8 30 28"));
    const ::rust::Vec<int32_t> rect = pictura::layer_rect_extents(*view, layer);
    QCOMPARE(int(rect.size()), 4);
    QCOMPARE(QList<int>({rect[0], rect[1], rect[2], rect[3]}), QList<int>({10, 8, 30, 28}));
}

// [ft_preview_stack] a resized lower layer stays under the layers above it
void TransformControlsTest::lowerLayerStaysUnderTheLayersAbove()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("TransformStack"), 120, 100, QStringLiteral("rgb"),
                              8, QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(view && canvas);

    const auto square = [view](int above, const QRect& box, QRgb colour) {
        const int index = view->add_layer(above);
        if (index < 0) {
            return QString();
        }
        const QString path = QString::number(index);
        view->set_active_layer(path);
        if (!view->select_rect(box.x(), box.y(), box.width(), box.height(),
                               QStringLiteral("new"), 0.0)
            || !pictura::edit_fill(*view, colour, -1, QStringLiteral("normal"), 100, false)) {
            return QString();
        }
        view->deselect();
        return path;
    };
    const QString blue = square(0, QRect(20, 20, 40, 40), qRgba(0, 0, 255, 255));
    QVERIFY2(!blue.isEmpty(), "the lower square");
    const QString red = square(blue.toInt(), QRect(40, 40, 40, 40), qRgba(255, 0, 0, 255));
    QVERIFY2(!red.isEmpty(), "the upper square");

    frame.selectLayerPath(blue);
    frame.setActiveTool(pictura::ToolId::Move);
    auto* box = frame.findChild<QCheckBox*>(QStringLiteral("optionsMoveTransform"));
    QVERIFY(box);
    box->setChecked(true);
    canvas->setSmartGuidesEnabled(false);
    QCoreApplication::processEvents();

    // Scale the blue from its bottom-right handle until it covers the red.
    canvas->mousePressed(QPointF(60.0, 60.0), Qt::LeftButton, int(Qt::NoModifier));
    QVERIFY(canvas->transformPreviewActive());
    QVERIFY2(!view->move_preview_above().isNull(), "the layers above are prepared");
    canvas->mouseMoved(QPointF(100.0, 100.0));

    const QImage shot = canvas->grab().toImage();
    const auto at = [&](double x, double y) {
        return shot.pixelColor(canvas->imageToWidget(QPointF(x, y)).toPoint());
    };
    QCOMPARE(at(55.0, 50.0), QColor(Qt::red));   // the layer above stays on top
    QCOMPARE(at(90.0, 90.0), QColor(Qt::blue));  // the resized content shows
    canvas->mouseReleased(QPointF(100.0, 100.0));
}

// [ft_preview_top] the topmost layer prepares no above-layer image
void TransformControlsTest::topmostLayerBuildsNoAboveImage()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(view && canvas);

    canvas->setSmartGuidesEnabled(false);
    canvas->mousePressed(QPointF(30.0, 28.0), Qt::LeftButton, int(Qt::NoModifier));
    QVERIFY(canvas->transformPreviewActive());
    QVERIFY(view->move_preview_above().isNull());
    canvas->mouseMoved(QPointF(36.0, 34.0));
    canvas->mouseReleased(QPointF(36.0, 34.0));
}

// [ft_preview_edge] the live preview anti-aliases the resampled edge
void TransformControlsTest::previewEdgeMatchesTheCommittedResample()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("PreviewEdge"), 100, 100, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && canvas && tools);
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    const int added = view->add_layer(0);
    QVERIFY(added > 0);
    const QString layer = QString::number(added);
    frame.selectLayerPath(layer);
    QVERIFY(view->select_rect(20, 20, 40, 40, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    view->deselect();
    canvas->setLayerEdgesVisible(false);
    canvas->setZoom(1.0, QPointF(50.0, 50.0));

    // An axis-aligned scale keeps the hard 0.3.0 edge; a gray intermediate
    // row is the 0.4.0 regression.
    QVERIFY(tools->beginFreeTransform(layer));
    QCoreApplication::processEvents();
    canvas->mousePressed(QPointF(40.0, 60.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(QPointF(40.0, 87.5));
    canvas->mouseReleased(QPointF(40.0, 87.5));
    QVERIFY(canvas->transformPreviewActive());
    // The box and handles sit on the edge; hide them so the sample is content.
    canvas->setTransformQuad(QPolygonF());
    QCoreApplication::processEvents();

    const auto edgeIsHard = [canvas](const QImage& shot) {
        for (int y = 80; y <= 95; ++y) {
            const int gray =
                qGray(shot.pixel(canvas->imageToWidget(QPointF(25.0, y + 0.5)).toPoint()));
            if (gray != 0 && gray != 255) {
                return false;
            }
        }
        return true;
    };
    QVERIFY2(edgeIsHard(canvas->grab().toImage()), "the preview edge is hard");
    tools->commitFreeTransform();
    QCoreApplication::processEvents();
    QVERIFY2(edgeIsHard(canvas->grab().toImage()), "the committed edge is hard");
}

// [ds_preview_one_teardown] every way a transform ends clears the same preview
// state. A preview is built by the begin; the ending must leave no cached base,
// moving, or above-layer image and no base origin.
void TransformControlsTest::everyTransformEndingClearsTheSamePreviewFields()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Endings"), 100, 100, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);

    // Two filled raster layers, so a begin can replace another session.
    view->select_all();
    QVERIFY(pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    view->deselect();
    QVERIFY(view->add_layer(0) > 0);
    const QString lower = QStringLiteral("1");
    QVERIFY(view->select_rect(10, 10, 20, 20, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, qRgba(255, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    view->deselect();

    // The observable preview set. The preview level is not observable here: the
    // scale readout is 1 whenever no session is live. The Rust test
    // `a_replaced_document_clears_the_preview_fields` asserts the level itself.
    const auto cleared = [](const pictura::PictureView* v) {
        return v->move_preview_base().isNull() && v->move_preview_above().isNull()
            && v->move_preview_layer().isNull() && pictura::move_preview_base_origin(*v).empty();
    };
    const auto built = [view](const QString& path) {
        if (!view->begin_free_transform(path)) {
            return false;
        }
        QCoreApplication::processEvents();
        return !view->move_preview_layer().isNull()
            || !view->move_preview_base().isNull();
    };

    // 1. An explicit cancel.
    QVERIFY2(built(lower), "the preview is built");
    QVERIFY2(!cleared(view), "a live session holds its preview");
    view->cancel_transform();
    QCoreApplication::processEvents();
    QVERIFY2(cleared(view), "cancel clears the preview");

    // 2. A commit that changed the document.
    QVERIFY2(built(lower), "the preview is built");
    QVERIFY(pictura::transform_set_angle(*view, 15.0));
    QVERIFY(view->commit_transform());
    QCoreApplication::processEvents();
    QVERIFY2(cleared(view), "a changed commit clears the preview");

    // 3. A commit refused for memory. A rotation keeps the session off the
    // identity path, so the commit reaches the governor and is refused there.
    QVERIFY2(built(QStringLiteral("1")), "the preview is built");
    QVERIFY(pictura::transform_set_angle(*view, 15.0));
    pictura::set_available_memory_for_test(0);
    const bool refusedCommit = view->commit_transform();
    pictura::set_available_memory_for_test(-1);
    QVERIFY2(!refusedCommit, "the commit is refused");
    QVERIFY2(pictura::edit_refused_for_memory(), "the refusal is a memory refusal");
    QCoreApplication::processEvents();
    QVERIFY2(cleared(view), "a refused commit clears the preview");

    // 4. A failed begin that replaced a live session: the outgoing preview must
    // not survive the refusal. The refusal comes from the preview's memory gate,
    // after `prepare_transform` has already taken the outgoing session.
    QVERIFY2(tools->beginFreeTransform(QStringLiteral("1")), "the preview is built");
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(canvas && canvas->transformPreviewActive());
    pictura::set_available_memory_for_test(0);
    const bool refusedBegin = !tools->beginFreeTransform(QStringLiteral("0"));
    pictura::set_available_memory_for_test(-1);
    QVERIFY2(refusedBegin, "the begin is refused");
    QCoreApplication::processEvents();
    QVERIFY2(cleared(view), "a failed begin clears the replaced session's preview");
    QVERIFY2(!canvas->transformPreviewActive(), "the failed begin clears the C++ overlay");

    // A placed smart object, for the two endings that only its vector path and
    // its rect-commit path reach.
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString source = dir.filePath(QStringLiteral("placed.png"));
    QImage seed(40, 30, QImage::Format_RGB32);
    seed.fill(QColor(30, 120, 220));
    QVERIFY(seed.save(source, "PNG"));
    const QString placed = view->place_image(source);
    QVERIFY2(!placed.isEmpty(), "the source is placed");
    view->cancel_transform();
    frame.setActiveTool(pictura::ToolId::Move);
    QCoreApplication::processEvents();

    // 5. A smart-object commit: an axis-aligned scale commits by moving the
    // layer rect, with no resample and no worker.
    QVERIFY2(built(placed), "the placed preview is built");
    QVERIFY(pictura::transform_set_width(*view, 200.0, true));
    QVERIFY(view->commit_transform());
    QCoreApplication::processEvents();
    QVERIFY2(cleared(view), "a smart-object commit clears the preview");

    // 6. A deferred commit that lands. The engine defers only past the area
    // threshold, so this leg needs a layer larger than it: a fresh document.
    QVERIFY(frame.newDocument(QStringLiteral("EndingsDeferred"), 2048, 2048,
                              QStringLiteral("rgb"), 8, QStringLiteral("white")));
    view = frame.activeView();
    QVERIFY(view);
    QVERIFY(view->begin_free_transform(QStringLiteral("0")));
    QVERIFY(pictura::transform_set_angle(*view, 15.0));
    QVERIFY(view->commit_transform());
    QVERIFY2(pictura::transform_pending(*view), "the commit runs deferred");
    QVERIFY(pictura::transform_commit_finish(*view));
    QCoreApplication::processEvents();
    QVERIFY2(cleared(view), "a landed deferred commit clears the preview");
}

// [ds_controls_flag_derived] the predicate is derived, not stored: idle box →
// handle gesture → changed release (pending) → cancel (idle box rebuilt).
void TransformControlsTest::controlsSessionFlagIsDerived()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && canvas && tools);

    // The idle box is a controls session with no live transform.
    QVERIFY2(tools->transformControlsSession(), "the idle box is a controls session");
    QVERIFY2(tools->transformControlsIdleForTest(), "and it is the idle one");
    QVERIFY2(!view->transform_session_active(), "no transform is live");
    QVERIFY2(!tools->transformPending(), "and none is pending");

    // A handle gesture: the box owns a live per-gesture session.
    canvas->mousePressed(QPointF(10.0, 8.0), Qt::LeftButton, int(Qt::NoModifier));
    QVERIFY2(view->transform_session_active(), "the gesture started a session");
    QVERIFY2(tools->transformControlsSession(), "still a controls session");
    QVERIFY2(!tools->transformPending(), "not pending while the gesture is held");

    // A changed release promotes the transform to pending: the box is no
    // longer the controls session, and the transform is now pending.
    canvas->mouseMoved(QPointF(4.0, 2.0));
    canvas->mouseReleased(QPointF(4.0, 2.0));
    QVERIFY2(view->transform_session_active(), "the session outlives the release");
    QVERIFY2(!tools->transformControlsSession(), "a pending transform is not a controls session");
    QVERIFY2(tools->transformPending(), "the transform is pending");

    // Cancel: no session, nothing pending, and the idle box is rebuilt, so the
    // predicate is true again for the reason it was true at the start.
    tools->cancelFreeTransform();
    QCoreApplication::processEvents();
    QVERIFY2(!view->transform_session_active(), "the session ended");
    QVERIFY2(!tools->transformPending(), "nothing is pending");
    QVERIFY2(tools->transformControlsSession(), "the idle box is back");
    QVERIFY2(tools->transformControlsIdleForTest(), "and it is idle again");
}

void TransformControlsTest::directTransformLifecycleOwnsTheControlsFlag()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(view && canvas);

    canvas->mousePressed(QPointF(10.0, 8.0), Qt::LeftButton, int(Qt::NoModifier));
    QVERIFY(view->transform_session_active());
    QVERIFY(pictura::transform_api::controlsSessionActive(*view));

    QVERIFY(!view->begin_free_transform(QStringLiteral("missing")));
    QVERIFY(view->transform_session_active());
    QVERIFY(pictura::transform_api::controlsSessionActive(*view));

    view->commit_transform();
    QVERIFY(!view->transform_session_active());
    QVERIFY(!pictura::transform_api::controlsSessionActive(*view));
    canvas->clearTransformPreview();
    canvas->mouseReleased(QPointF(10.0, 8.0));
}

// [ds_controls_flag_move_path] Move previews preserve a live controls flag and
// leave a pending transform pending.
void TransformControlsTest::movePreviewKeepsAPromotedTransform()
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
    QVERIFY2(pictura::transform_api::controlsSessionActive(*view),
             "the live gesture owns the controls flag");
    QVERIFY2(view->begin_move_preview(), "a Move preview begins during the controls gesture");
    QVERIFY2(pictura::transform_api::controlsSessionActive(*view),
             "the Move preview preserves the live controls flag");
    QVERIFY2(tools->transformControlsSession(), "the live gesture is still a controls session");
    view->end_move_preview();
    QVERIFY2(pictura::transform_api::controlsSessionActive(*view),
             "ending the Move preview preserves the live controls flag");

    canvas->mouseMoved(QPointF(4.0, 2.0));
    canvas->mouseReleased(QPointF(4.0, 2.0));
    QVERIFY2(tools->transformPending(), "the released gesture is pending");

    // Start a Move preview over the pending session's target.
    QVERIFY2(view->begin_move_preview(), "a Move preview begins");
    QVERIFY2(!pictura::transform_api::controlsSessionActive(*view),
             "the Move preview did not clear the transform's controls flag");
    QVERIFY2(tools->transformPending(), "the transform is still pending");

    // Its own cancel path leaves the pending transform alone too.
    view->end_move_preview();
    QVERIFY2(tools->transformPending(), "ending the Move preview leaves it pending");
    QCOMPARE(view->history_count(), base);

    tools->cancelFreeTransform();
    QCOMPARE(view->history_count(), base);
}

QTEST_MAIN(TransformControlsTest)
#include "tst_transform_controls.moc"
