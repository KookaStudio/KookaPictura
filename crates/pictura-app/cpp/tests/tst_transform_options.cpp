// The Free Transform options bar (#296): the session-keyed page, its numeric
// setters, the interpolation menu, and the canvas reference point drag (Warp
// mode is tst_transform_warp).

#include <QtTest/QtTest>

#include "api/transform_api.h"
#include "frame.h"
#include "image_view.h"
#include "options_bar.h"
#include "panels/numeric_field.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_transform/bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include "qt_test_support.h"

#include <QtWidgets/QComboBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QToolButton>

namespace {

constexpr double kTolerance = 1e-6;

} // namespace

class TransformOptionsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void pageSwitchesWithTheSession();
    void fieldsSeedFromTheSession();
    void pivotLocatorMovesTheReferencePoint();
    void relativePositioningOffsetsFromTheOrigin();
    void widthLinkScalesBothAxes();
    void angleAndSkewSettersReachTheSession();
    void interpolationComboReachesTheSession();
    void canvasPivotDragSnapsToAnAnchor();
    void perimeterRotatesAndFarPressDoesNotCommit();
    void applyCommitsCancelRestores();

private:
    // A new document with one filled raster layer selected; returns its path.
    QString beginLayer(pictura::PicturaMainWindow& frame);
    static void typeInto(pictura::NumericField* field, const QString& text);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void TransformOptionsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void TransformOptionsTest::cleanup()
{
    while (window_ && window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
}

QString TransformOptionsTest::beginLayer(pictura::PicturaMainWindow& frame)
{
    if (!frame.newDocument(QStringLiteral("Transform"), 40, 30, QStringLiteral("rgb"), 8,
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

void TransformOptionsTest::typeInto(pictura::NumericField* field, const QString& text)
{
    auto* edit = field->findChild<QLineEdit*>();
    QVERIFY(edit);
    edit->clear();
    QTest::keyClicks(edit, text);
    QTest::keyClick(edit, Qt::Key_Return);
}

void TransformOptionsTest::pageSwitchesWithTheSession()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* bar = frame.findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && bar && tools);
    QVERIFY(bar->transformPageForTest());

    QCOMPARE(bar->currentPageForTest(), bar->pageForTest(tools->activeTool()));
    QVERIFY(tools->beginFreeTransform(layer));
    QCOMPARE(bar->currentPageForTest(), bar->transformPageForTest());

    tools->cancelFreeTransform();
    QCOMPARE(bar->currentPageForTest(), bar->pageForTest(tools->activeTool()));

    // A tool switch cancels the session and restores the new tool's page.
    QVERIFY(tools->beginFreeTransform(layer));
    QCOMPARE(bar->currentPageForTest(), bar->transformPageForTest());
    tools->setActiveTool(pictura::ToolId::Brush);
    QVERIFY(!view->transform_session_active());
    QCOMPARE(bar->currentPageForTest(), bar->pageForTest(pictura::ToolId::Brush));
}

void TransformOptionsTest::fieldsSeedFromTheSession()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools && tools->beginFreeTransform(layer));

    auto* x = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsTransformX"));
    auto* y = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsTransformY"));
    auto* w = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsTransformW"));
    auto* h = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsTransformH"));
    auto* angle = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsTransformAngle"));
    auto* interpolation =
        frame.findChild<QComboBox*>(QStringLiteral("optionsTransformInterpolation"));
    QVERIFY(x && y && w && h && angle && interpolation);

    // The filled square is (10,8)-(22,17): the pivot starts at its centre.
    QVERIFY(std::abs(x->value() - 16.0) < kTolerance);
    QVERIFY(std::abs(y->value() - 12.5) < kTolerance);
    QVERIFY(std::abs(w->value() - 100.0) < kTolerance);
    QVERIFY(std::abs(h->value() - 100.0) < kTolerance);
    QVERIFY(std::abs(angle->value()) < kTolerance);
    QCOMPARE(interpolation->currentData().toString(), QStringLiteral("bicubic"));

    // A canvas gesture refreshes the fields through transformChanged.
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(canvas);
    canvas->mousePressed(QPointF(10.0, 8.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(QPointF(4.0, 2.0));
    canvas->mouseReleased(QPointF(4.0, 2.0));
    QVERIFY(w->value() > 100.0);
    tools->cancelFreeTransform();
}

void TransformOptionsTest::pivotLocatorMovesTheReferencePoint()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools && tools->beginFreeTransform(layer));

    auto* cell = frame.findChild<QToolButton*>(QStringLiteral("optionsPivot0"));
    QVERIFY(cell);
    cell->click();
    QVERIFY(std::abs(pictura::transform_pivot_x(*view) - 10.0) < kTolerance);
    QVERIFY(std::abs(pictura::transform_pivot_y(*view) - 8.0) < kTolerance);
    // The quad must not move when the pivot does.
    QCOMPARE(pictura::transform_api::quad(*view).toString(),
             QStringLiteral("10.00,8.00 22.00,8.00 22.00,17.00 10.00,17.00"));

    // The canvas marker tracks the pivot and hides with the toggle.
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(canvas);
    QVERIFY(canvas->transformPivotVisibleForTest());
    QVERIFY(std::abs(canvas->transformPivotForTest().x() - 10.0) < kTolerance);
    QVERIFY(std::abs(canvas->transformPivotForTest().y() - 8.0) < kTolerance);

    // The show toggle hides the marker without ending the session.
    auto* show = frame.findChild<QToolButton*>(QStringLiteral("optionsTransformShowPivot"));
    QVERIFY(show);
    QVERIFY(show->isChecked());
    show->click();
    QVERIFY(!pictura::transform_show_pivot(*view));
    QVERIFY(!canvas->transformPivotVisibleForTest());
    show->click();
    QVERIFY(pictura::transform_show_pivot(*view));
    QVERIFY(canvas->transformPivotVisibleForTest());

    tools->cancelFreeTransform();
}

void TransformOptionsTest::relativePositioningOffsetsFromTheOrigin()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools && tools->beginFreeTransform(layer));

    auto* x = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsTransformX"));
    auto* relative = frame.findChild<QToolButton*>(QStringLiteral("optionsTransformRelative"));
    QVERIFY(x && relative);
    relative->setChecked(true);
    QVERIFY(std::abs(x->value()) < kTolerance);

    typeInto(x, QStringLiteral("10"));
    QVERIFY(std::abs(pictura::transform_pivot_x(*view) - 26.0) < kTolerance);
    QVERIFY(std::abs(x->value() - 10.0) < kTolerance);

    relative->setChecked(false);
    QVERIFY(std::abs(x->value() - 26.0) < kTolerance);
    tools->cancelFreeTransform();
}

void TransformOptionsTest::widthLinkScalesBothAxes()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools && tools->beginFreeTransform(layer));

    auto* w = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsTransformW"));
    auto* h = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsTransformH"));
    auto* link = frame.findChild<QToolButton*>(QStringLiteral("optionsTransformLink"));
    QVERIFY(w && h && link);
    QVERIFY(link->isChecked());

    typeInto(w, QStringLiteral("150"));
    QVERIFY(std::abs(pictura::transform_width_pct(*view) - 150.0) < kTolerance);
    QVERIFY(std::abs(pictura::transform_height_pct(*view) - 150.0) < kTolerance);
    QVERIFY(std::abs(h->value() - 150.0) < kTolerance);

    // Unlinking leaves H alone.
    link->setChecked(false);
    typeInto(h, QStringLiteral("200"));
    QVERIFY(std::abs(pictura::transform_width_pct(*view) - 150.0) < kTolerance);
    QVERIFY(std::abs(pictura::transform_height_pct(*view) - 200.0) < kTolerance);
    tools->cancelFreeTransform();
}

void TransformOptionsTest::angleAndSkewSettersReachTheSession()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools && tools->beginFreeTransform(layer));

    auto* angle = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsTransformAngle"));
    auto* skewH = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsTransformSkewH"));
    auto* skewV = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsTransformSkewV"));
    QVERIFY(angle && skewH && skewV);

    typeInto(angle, QStringLiteral("30"));
    QVERIFY(std::abs(pictura::transform_angle_degrees(*view) - 30.0) < 1e-3);

    typeInto(skewH, QStringLiteral("15"));
    typeInto(skewV, QStringLiteral("-10"));
    QVERIFY(std::abs(pictura::transform_skew_h_degrees(*view) - 15.0) < 1e-3);
    QVERIFY(std::abs(pictura::transform_skew_v_degrees(*view) + 10.0) < 1e-3);

    // A sheared similarity commits through the affine quad as one state.
    const int base = view->history_count();
    tools->commitFreeTransform();
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Free Transform"));
}

void TransformOptionsTest::interpolationComboReachesTheSession()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools && tools->beginFreeTransform(layer));

    auto* combo = frame.findChild<QComboBox*>(QStringLiteral("optionsTransformInterpolation"));
    QVERIFY(combo);
    QCOMPARE(combo->count(), 6);
    combo->setCurrentIndex(combo->findData(QStringLiteral("nearest")));
    QCOMPARE(pictura::transform_interpolation_id(*view), QStringLiteral("nearest"));
    combo->setCurrentIndex(combo->findData(QStringLiteral("bicubic-automatic")));
    QCOMPARE(pictura::transform_interpolation_id(*view), QStringLiteral("bicubic-automatic"));
    tools->cancelFreeTransform();
}

void TransformOptionsTest::canvasPivotDragSnapsToAnAnchor()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools && tools->beginFreeTransform(layer));

    // Press on the pivot (the rect centre), drag near the top-left anchor.
    const int hit = view->transform_press(16.0, 12.5, 1.0, false, false);
    QCOMPARE(hit, 10);
    QVERIFY(view->transform_move(10.5, 8.5, 1.0, false, false));
    QVERIFY(std::abs(pictura::transform_pivot_x(*view) - 10.0) < kTolerance);
    QVERIFY(std::abs(pictura::transform_pivot_y(*view) - 8.0) < kTolerance);
    view->transform_release();
    QCOMPARE(pictura::transform_api::quad(*view).toString(),
             QStringLiteral("10.00,8.00 22.00,8.00 22.00,17.00 10.00,17.00"));

    // A drag away from every anchor keeps the pointer's position.
    QVERIFY(view->transform_press(10.0, 8.0, 1.0, false, false) == 10);
    QVERIFY(view->transform_move(3.5, 20.0, 1.0, false, false));
    QVERIFY(std::abs(pictura::transform_pivot_x(*view) - 3.5) < kTolerance);
    QVERIFY(std::abs(pictura::transform_pivot_y(*view) - 20.0) < kTolerance);
    view->transform_release();
    tools->cancelFreeTransform();
}

void TransformOptionsTest::perimeterRotatesAndFarPressDoesNotCommit()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && canvas && tools && tools->beginFreeTransform(layer));

    // The filled square is (10,8)-(22,17); just outside the top edge is in the
    // rotate band, not only the corners.
    QCOMPARE(view->transform_hit_test(16.0, 0.0, 1.0), 8);
    canvas->mousePressed(QPointF(16.0, 0.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(QPointF(26.0, 4.0));
    canvas->mouseReleased(QPointF(26.0, 4.0));
    QVERIFY(std::abs(pictura::transform_angle_degrees(*view)) > 1.0);

    // A press beyond the band hits nothing, commits nothing, and leaves the
    // session running.
    const int history = view->history_count();
    canvas->mousePressed(QPointF(16.0, -40.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseReleased(QPointF(16.0, -40.0));
    QVERIFY(view->transform_session_active());
    QCOMPARE(view->history_count(), history);
    tools->cancelFreeTransform();
}

void TransformOptionsTest::applyCommitsCancelRestores()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools && tools->beginFreeTransform(layer));
    auto* angle = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsTransformAngle"));
    auto* apply = frame.findChild<QToolButton*>(QStringLiteral("optionsTransformApply"));
    auto* cancel = frame.findChild<QToolButton*>(QStringLiteral("optionsTransformCancel"));
    QVERIFY(angle && apply && cancel);

    // Cancel restores the document exactly.
    typeInto(angle, QStringLiteral("45"));
    const int base = view->history_count();
    cancel->click();
    QVERIFY(!view->transform_session_active());
    QCOMPARE(view->history_count(), base);

    // Apply commits one "Free Transform" state.
    QVERIFY(tools->beginFreeTransform(layer));
    typeInto(angle, QStringLiteral("30"));
    apply->click();
    QVERIFY(!view->transform_session_active());
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Free Transform"));
}

QTEST_MAIN(TransformOptionsTest)
#include "tst_transform_options.moc"
