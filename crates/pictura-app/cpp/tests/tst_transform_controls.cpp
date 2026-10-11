// The Move tool's Show Transform Controls box (#296 follow-up): the box shown
// on the active layer, its per-drag handle gestures, and the Move drags that
// pass through it.

#include <QtTest/QtTest>

#include "frame.h"
#include "image_view.h"
#include "options_bar.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_transform/bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include "qt_test_support.h"

#include <QtWidgets/QCheckBox>

class TransformControlsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void checkingTheBoxShowsControls();
    void boxFollowsTheActiveLayer();
    void handleDragScalesAndCommits();
    void insideDragMovesAndRecordsMove();
    void clickInsideLeavesTheDocumentAlone();
    void selectionSuppressesTheBox();
    void backgroundAndToolSwitchSuppressTheBox();
    void ctrlTTakesTheBoxOver();

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

    QVERIFY(tools->transformControlsSession());
    QVERIFY(view->transform_session_active());
    QCOMPARE(view->transform_session_path(), layer);
    QVERIFY(canvas->transformPreviewActive());
    // CS6's controls box has no reference point.
    QVERIFY(!canvas->transformPivotVisibleForTest());
    // The bar stays on the Move page.
    QCOMPARE(bar->currentPageForTest(), bar->pageForTest(pictura::ToolId::Move));
    // The box hugs the filled square.
    QCOMPARE(pictura::transform_session_rect(*view), QStringLiteral("10 8 30 28"));

    // Unchecking hides the box without recording anything.
    const int history = view->history_count();
    auto* box = frame.findChild<QCheckBox*>(QStringLiteral("optionsMoveTransform"));
    QVERIFY(box);
    box->setChecked(false);
    QVERIFY(!view->transform_session_active());
    QVERIFY(!canvas->transformPreviewActive());
    QCOMPARE(view->history_count(), history);
}

void TransformControlsTest::boxFollowsTheActiveLayer()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString first = beginControls(frame, true);
    QVERIFY(!first.isEmpty());
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    QVERIFY(view->transform_session_active());

    const int added = view->add_layer(0);
    QVERIFY(added > 0);
    const QString second = QString::number(added);
    frame.selectLayerPath(second);
    QVERIFY(view->select_rect(2, 2, 6, 6, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, qRgba(255, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    view->deselect();

    // The box rebuilt on the new layer's content rect.
    QCOMPARE(view->transform_session_path(), second);
    QCOMPARE(pictura::transform_session_rect(*view), QStringLiteral("2 2 8 8"));
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
    // The box returns on the scaled layer.
    QVERIFY(tools->transformControlsSession());
    QVERIFY(view->transform_session_active());
    QCOMPARE(pictura::transform_session_rect(*view), QStringLiteral("4 2 30 28"));
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
    canvas->mousePressed(QPointF(15.0, 13.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(QPointF(20.0, 16.0));
    canvas->mouseReleased(QPointF(20.0, 16.0));

    // An inside drag is the Move tool's, not the session's.
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Move Layer"));
    QVERIFY(tools->transformControlsSession());
    QCOMPARE(pictura::transform_session_rect(*view), QStringLiteral("15 11 35 31"));
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
    QVERIFY(tools->transformControlsSession());
    QVERIFY(view->transform_session_active());
}

void TransformControlsTest::selectionSuppressesTheBox()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    QVERIFY(view->transform_session_active());

    QVERIFY(view->select_rect(5, 5, 6, 6, QStringLiteral("new"), 0.0));
    QVERIFY(!view->transform_session_active());

    view->deselect();
    QVERIFY(view->transform_session_active());
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
    frame.closeDocument(frame.activeDocumentIndex(), false);

    // A tool switch hides a shown box; returning to Move shows it again.
    const QString layer = beginControls(frame, true);
    QVERIFY(!layer.isEmpty());
    view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools && tools->transformControlsSession());
    frame.setActiveTool(pictura::ToolId::Brush);
    QVERIFY(!view->transform_session_active());
    frame.setActiveTool(pictura::ToolId::Move);
    QVERIFY(tools->transformControlsSession());
    QVERIFY(view->transform_session_active());
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

QTEST_MAIN(TransformControlsTest)
#include "tst_transform_controls.moc"
