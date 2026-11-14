// Select > Transform Selection (#526): the shared transform box on the
// selection border, the mask commit and its undo, the live ant preview, Warp,
// Transform Again, and the modal lock.

#include <QtTest/QtTest>

#include "api/history_api.h"
#include "api/selection_api.h"
#include "api/transform_api.h"
#include "commands.h"
#include "dialogs.h"
#include "frame.h"
#include "image_view.h"
#include "options_bar.h"
#include "panels/numeric_field.h"
#include "tools.h"

#include "api/paint_bridge.h"
#include "api/transform_bridge.h"

#include "qt_test_support.h"

#include <QtGui/QAction>
#include <QtGui/QPolygonF>
#include <QtWidgets/QToolButton>

#include <cmath>

namespace {

constexpr double kTolerance = 1e-6;
const QRect kBlock(10, 8, 12, 9);

QString beginDocument(pictura::PicturaMainWindow& frame)
{
    if (!frame.newDocument(QStringLiteral("Transform Selection"), 40, 30,
                           QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
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
    return layer;
}

bool selectBlock(pictura::PictureView* view, const QRect& block = kBlock)
{
    return view->select_rect(block.x(), block.y(), block.width(), block.height(),
                             QStringLiteral("new"), 0.0);
}

QByteArray coverageBytes(pictura::PictureView* view)
{
    const pictura::selection_api::SelectionBounds bounds = pictura::selection_api::bounds(*view);
    QByteArray out;
    if (!bounds.valid) {
        return out;
    }
    for (int y = bounds.y; y < bounds.y + bounds.height; ++y) {
        for (int x = bounds.x; x < bounds.x + bounds.width; ++x) {
            out.append(char(view->selection_coverage(x, y)));
        }
    }
    return out;
}

QRect quadBox(const QPolygonF& quad)
{
    return quad.boundingRect().toAlignedRect();
}

} // namespace

class TransformSelectionTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void menuEnabledWithSelection();
    void menuDisabledWithoutSelection();
    void menuDisabledWhileModal();
    void beginPlacesTheBoxOnTheSelectionBounds();
    void scaleWithShiftCommitsProportionally();
    void widthFieldDoublesTheBorder();
    void rotationKeepsTheReferenceCorner();
    void commitRecordsOneStateAndReplacesTheBorder();
    void commitLeavesPixelsUntouched();
    void undoRestoresThePreviousBorder();
    void identityCommitRecordsNothing();
    void offCanvasCommitClearsTheSelection();
    void softCoverageStaysSoft();
    void escapeRestoresTheBorder();
    void previewFollowsTheDrag();
    void committedOutlineMatchesTheMask();
    void cancelKeepsHistoryAndSelection();
    void warpEntersOnTheSelection();
    void warpPresetAndDragReshapeThePreview();
    void warpCommitResamplesTheMask();
    void warpLeaveKeepsTheBorder();
    void warpComposesWithPerspective();
    void ctrlTStillLiftsPixels();
    void borderTransformKeepsTheSelection();
    void selectMenuLockedWhilePending();
    void toolSwitchPromptsForTheBorder();
    void fieldsSeedFromTheSelectionBounds();
    void applyRecordsTransformSelection();
    void againRepeatsABorderTransform();
    void warpKeepsAnOpenBorderSession();

private:
    QAction* action(const char* id) const;

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void TransformSelectionTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void TransformSelectionTest::cleanup()
{
    if (window_) {
        if (auto* tools = window_->findChild<pictura::ToolController*>()) {
            tools->cancelFreeTransform();
            tools->setTransformDecisionPrompt(nullptr);
        }
        window_->setActiveTool(pictura::ToolId::Move);
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
}

QAction* TransformSelectionTest::action(const char* id) const
{
    window_->registry()->refresh();
    return window_->registry()->action(QString::fromLatin1(id));
}

void TransformSelectionTest::menuEnabledWithSelection()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    QVERIFY(selectBlock(frame.activeView()));
    QAction* transform = action(pictura::command_ids::SelectTransformSelection);
    QVERIFY(transform);
    QVERIFY(transform->isEnabled());
}

void TransformSelectionTest::menuDisabledWithoutSelection()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    QVERIFY(!frame.activeView()->has_selection());
    QAction* transform = action(pictura::command_ids::SelectTransformSelection);
    QVERIFY(transform);
    QVERIFY(!transform->isEnabled());
}

void TransformSelectionTest::menuDisabledWhileModal()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginDocument(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));
    QVERIFY(pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    QVERIFY(tools->beginFreeTransform(layer));
    QAction* transform = action(pictura::command_ids::SelectTransformSelection);
    QVERIFY(transform);
    QVERIFY(!transform->isEnabled());
    QVERIFY(view->transform_session_active());
    tools->cancelFreeTransform();
}

void TransformSelectionTest::beginPlacesTheBoxOnTheSelectionBounds()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* bar = frame.findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(view && tools && bar);
    QVERIFY(selectBlock(view));
    const int history = view->history_count();
    QVERIFY(tools->beginTransformSelection());
    QVERIFY(view->transform_session_active());
    QCOMPARE(view->history_count(), history);
    QCOMPARE(quadBox(pictura::transform_api::quad(*view).corners), kBlock);
    QCOMPARE(bar->currentPageForTest(), bar->transformPageForTest());
    tools->cancelFreeTransform();
}

void TransformSelectionTest::scaleWithShiftCommitsProportionally()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));
    const QRect before = kBlock;
    QVERIFY(tools->beginTransformSelection());
    const int history = view->history_count();
    QCOMPARE(view->transform_press(before.right() + 1.0, before.bottom() + 1.0, 1.0, false,
                                   false),
             2);
    QVERIFY(view->transform_move(before.right() + 7.0, before.bottom() + 6.0, 1.0, true,
                                 false));
    view->transform_release();
    tools->commitFreeTransform();
    QVERIFY(!view->transform_session_active());
    QCOMPARE(view->history_count(), history + 1);
    const pictura::selection_api::SelectionBounds after =
        pictura::selection_api::bounds(*view);
    QVERIFY(after.valid);
    const double ratio = double(after.width) / double(after.height);
    QVERIFY(std::abs(ratio - double(before.width()) / double(before.height())) < 0.2);
}

void TransformSelectionTest::widthFieldDoublesTheBorder()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));
    QVERIFY(tools->beginTransformSelection());
    QVERIFY(tools->setTransformWidth(200.0, true));
    tools->commitFreeTransform();
    const pictura::selection_api::SelectionBounds after =
        pictura::selection_api::bounds(*view);
    QVERIFY(after.valid);
    QVERIFY(std::abs(after.width - kBlock.width() * 2) <= 1);
    QVERIFY(std::abs(after.height - kBlock.height() * 2) <= 1);
}

void TransformSelectionTest::rotationKeepsTheReferenceCorner()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));
    QVERIFY(tools->beginTransformSelection());
    QVERIFY(tools->setTransformPivotPreset(0));
    QVERIFY(tools->setTransformAngle(90.0));
    tools->commitFreeTransform();
    const pictura::selection_api::SelectionBounds after =
        pictura::selection_api::bounds(*view);
    QVERIFY(after.valid);
    QVERIFY(std::abs(after.width - kBlock.height()) <= 1);
    QVERIFY(std::abs(after.height - kBlock.width()) <= 1);
    const int right = after.x + after.width;
    const int bottom = after.y + after.height;
    const bool pivotOnX = std::abs(after.x - kBlock.x()) <= 1
        || std::abs(right - kBlock.x()) <= 1;
    const bool pivotOnY = std::abs(after.y - kBlock.y()) <= 1
        || std::abs(bottom - kBlock.y()) <= 1;
    QVERIFY(pivotOnX && pivotOnY);
}

void TransformSelectionTest::commitRecordsOneStateAndReplacesTheBorder()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));
    const QByteArray before = coverageBytes(view);
    QVERIFY(tools->beginTransformSelection());
    const int base = view->history_count();
    QVERIFY(tools->setTransformAngle(30.0));
    tools->commitFreeTransform();
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Transform Selection"));
    QVERIFY(coverageBytes(view) != before);
    QVERIFY(view->has_selection());
}

void TransformSelectionTest::commitLeavesPixelsUntouched()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginDocument(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(view && tools && canvas);
    QVERIFY(selectBlock(view, QRect(4, 4, 20, 18)));
    QVERIFY(pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    view->deselect();
    const QImage before = canvas->image();
    QVERIFY(selectBlock(view));
    QVERIFY(tools->beginTransformSelection());
    QVERIFY(tools->setTransformWidth(150.0, true));
    tools->commitFreeTransform();
    const QImage after = canvas->image();
    QCOMPARE(after.size(), before.size());
    QVERIFY(after == before);
}

void TransformSelectionTest::undoRestoresThePreviousBorder()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));
    const QByteArray before = coverageBytes(view);
    QVERIFY(tools->beginTransformSelection());
    QVERIFY(tools->setTransformWidth(180.0, true));
    tools->commitFreeTransform();
    QVERIFY(view->undo());
    QCOMPARE(coverageBytes(view), before);
}

void TransformSelectionTest::identityCommitRecordsNothing()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));
    const QByteArray before = coverageBytes(view);
    QVERIFY(tools->beginTransformSelection());
    const int history = view->history_count();
    tools->commitFreeTransform();
    QCOMPARE(view->history_count(), history);
    QCOMPARE(coverageBytes(view), before);
}

void TransformSelectionTest::offCanvasCommitClearsTheSelection()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));
    const QByteArray before = coverageBytes(view);
    QVERIFY(tools->beginTransformSelection());
    const int history = view->history_count();
    QCOMPARE(view->transform_press(11.0, 9.0, 1.0, false, false), 9);
    QVERIFY(view->transform_move(200.0, 200.0, 1.0, false, false));
    view->transform_release();
    tools->commitFreeTransform();
    QCOMPARE(view->history_count(), history + 1);
    QCOMPARE(view->selection_count(), 0);
    QAction* transform = action(pictura::command_ids::SelectTransformSelection);
    QAction* again = action(pictura::command_ids::EditTransformAgain);
    QVERIFY(transform && again);
    QVERIFY(!transform->isEnabled());
    QVERIFY(!again->isEnabled());
    QVERIFY(view->undo());
    QCOMPARE(coverageBytes(view), before);
}

void TransformSelectionTest::softCoverageStaysSoft()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));
    QVERIFY(view->modify_selection(QStringLiteral("feather"), 3.0));
    QVERIFY(tools->beginTransformSelection());
    QVERIFY(tools->setTransformWidth(150.0, true));
    tools->commitFreeTransform();
    int soft = 0;
    for (int y = 0; y < 30; ++y) {
        for (int x = 0; x < 40; ++x) {
            const int coverage = view->selection_coverage(x, y);
            soft += coverage > 0 && coverage < 255 ? 1 : 0;
        }
    }
    QVERIFY(soft > 0);
}

void TransformSelectionTest::escapeRestoresTheBorder()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && canvas && tools);
    QVERIFY(selectBlock(view));
    const QByteArray before = coverageBytes(view);
    const int history = view->history_count();
    QVERIFY(tools->beginTransformSelection());
    QVERIFY(tools->setTransformWidth(170.0, true));
    canvas->setFocus();
    QTest::keyClick(canvas, Qt::Key_Escape);
    QVERIFY(!view->transform_session_active());
    QCOMPARE(coverageBytes(view), before);
    QCOMPARE(view->history_count(), history);
}

void TransformSelectionTest::previewFollowsTheDrag()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && canvas && tools);
    QVERIFY(selectBlock(view));
    QVERIFY(tools->beginTransformSelection());
    const int history = view->history_count();
    canvas->mousePressed(QPointF(kBlock.right() + 1.0, kBlock.bottom() + 1.0),
                         Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(QPointF(kBlock.right() + 6.0, kBlock.bottom() + 4.0));
    QVERIFY(canvas->hasSelectionPreviewForTest());
    QVERIFY(!canvas->hasSelectionContourForTest());
    QCOMPARE(view->history_count(), history);
    canvas->mouseReleased(QPointF(kBlock.right() + 6.0, kBlock.bottom() + 4.0));
    tools->cancelFreeTransform();
}

void TransformSelectionTest::committedOutlineMatchesTheMask()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));
    QVERIFY(tools->beginTransformSelection());
    QVERIFY(tools->setTransformAngle(20.0));
    tools->commitFreeTransform();
    const pictura::selection_api::SelectionContour contour =
        pictura::selection_api::contour(*view);
    const pictura::selection_api::SelectionBounds bounds =
        pictura::selection_api::bounds(*view);
    QVERIFY(!contour.isEmpty());
    QVERIFY(bounds.valid);
    QRectF outline;
    bool first = true;
    for (const QPolygonF& loop : contour.loops) {
        outline = first ? loop.boundingRect() : outline.united(loop.boundingRect());
        first = false;
    }
    QVERIFY(std::abs(outline.left() - bounds.x) <= 1.0);
    QVERIFY(std::abs(outline.top() - bounds.y) <= 1.0);
    QVERIFY(std::abs(outline.right() - (bounds.x + bounds.width)) <= 1.0);
    QVERIFY(std::abs(outline.bottom() - (bounds.y + bounds.height)) <= 1.0);

    // The outline is the 50 % line: at some vertex the surrounding pixels
    // straddle the threshold.
    bool crosses = false;
    for (const QPolygonF& loop : contour.loops) {
        for (const QPointF& vertex : loop) {
            const int x = qRound(vertex.x());
            const int y = qRound(vertex.y());
            const int around[4][2] = {{x - 1, y}, {x, y}, {x - 1, y - 1}, {x, y - 1}};
            bool low = false;
            bool high = false;
            for (const auto& pixel : around) {
                if (pixel[0] < 0 || pixel[1] < 0 || pixel[0] >= view->document_width()
                    || pixel[1] >= view->document_height()) {
                    continue;
                }
                const int coverage = view->selection_coverage(pixel[0], pixel[1]);
                low = low || coverage < 128;
                high = high || coverage >= 128;
            }
            crosses = crosses || (low && high);
        }
    }
    QVERIFY(crosses);
}

void TransformSelectionTest::cancelKeepsHistoryAndSelection()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));
    const QByteArray before = coverageBytes(view);
    const int history = view->history_count();
    const quint64 revision = view->selection_revision();
    QVERIFY(tools->beginTransformSelection());
    QVERIFY(tools->setTransformWidth(160.0, true));
    QVERIFY(tools->setTransformAngle(25.0));
    tools->cancelFreeTransform();
    QCOMPARE(view->history_count(), history);
    QCOMPARE(coverageBytes(view), before);
    QCOMPARE(view->selection_revision(), revision);
}

void TransformSelectionTest::warpEntersOnTheSelection()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(view && tools && canvas);
    QVERIFY(selectBlock(view));
    QVERIFY(tools->beginTransformSelection());
    QVERIFY(tools->enterTransformWarp());
    QVERIFY(pictura::transform_warp_active(*view));
    QVERIFY(!canvas->transformWarpForTest().isEmpty());
    tools->cancelFreeTransform();
}

void TransformSelectionTest::warpPresetAndDragReshapeThePreview()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(view && tools && canvas);
    QVERIFY(selectBlock(view));
    QVERIFY(tools->beginTransformSelection());
    QVERIFY(tools->enterTransformWarp());
    const int history = view->history_count();
    QVERIFY(tools->setTransformWarpStyle(QStringLiteral("arc")));
    QVERIFY(!canvas->transformWarpForTest().isEmpty());
    QVERIFY(tools->setTransformWarpValues(60.0, 0.0, 0.0));
    QCOMPARE(view->history_count(), history);
    QVERIFY(canvas->hasSelectionPreviewForTest());

    QVERIFY(tools->setTransformWarpStyle(QStringLiteral("custom")));
    const QList<QPolygonF> before = pictura::transform_api::selectionContour(*view);
    QVERIFY(!before.isEmpty());
    QPointF anchor;
    bool found = false;
    for (const pictura::transform_api::WarpPoint& point
         : pictura::transform_api::warpOverlay(*view).points) {
        if (point.kind == pictura::transform_api::WarpPoint::Anchor) {
            anchor = point.pos;
            found = true;
            break;
        }
    }
    QVERIFY(found);
    QCOMPARE(view->transform_press(anchor.x(), anchor.y(), 1.0, false, false), 11);
    QVERIFY(view->transform_move(anchor.x() + 5.0, anchor.y() + 4.0, 1.0, false, false));
    view->transform_release();
    const QList<QPolygonF> after = pictura::transform_api::selectionContour(*view);
    QVERIFY(after != before);
    QCOMPARE(view->history_count(), history);
    tools->cancelFreeTransform();
}

void TransformSelectionTest::warpCommitResamplesTheMask()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));
    const QByteArray before = coverageBytes(view);
    QVERIFY(tools->beginTransformSelection());
    const int base = view->history_count();
    QVERIFY(tools->enterTransformWarp());
    QVERIFY(tools->setTransformWarpStyle(QStringLiteral("arc")));
    QVERIFY(tools->setTransformWarpValues(60.0, 0.0, 0.0));
    tools->commitFreeTransform();
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Transform Selection"));
    QVERIFY(coverageBytes(view) != before);
}

void TransformSelectionTest::warpLeaveKeepsTheBorder()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(view && tools && canvas);
    QVERIFY(selectBlock(view));
    QVERIFY(tools->beginTransformSelection());
    QVERIFY(tools->enterTransformWarp());
    QVERIFY(tools->setTransformWarpStyle(QStringLiteral("arc")));
    const QList<QPolygonF> before = pictura::transform_api::selectionContour(*view);
    QVERIFY(!before.isEmpty());
    const QPolygonF box = pictura::transform_api::quad(*view).corners;
    QVERIFY(tools->leaveTransformWarp());
    QVERIFY(pictura::transform_warp_stored(*view));
    QVERIFY(pictura::transform_api::quad(*view).corners != box);
    const QList<QPolygonF> after = pictura::transform_api::selectionContour(*view);
    QVERIFY(after == before);
    tools->cancelFreeTransform();
}

void TransformSelectionTest::warpComposesWithPerspective()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));
    const QByteArray before = coverageBytes(view);
    QVERIFY(tools->beginTransformSelection());
    QVERIFY(tools->setTransformMode(QStringLiteral("perspective")));
    QCOMPARE(view->transform_press(kBlock.right() + 1.0, kBlock.bottom() + 1.0, 1.0, false,
                                   false),
             2);
    QVERIFY(view->transform_move(kBlock.right() - 3.0, kBlock.bottom() + 4.0, 1.0, false,
                                 false));
    view->transform_release();
    QVERIFY(tools->enterTransformWarp());
    QVERIFY(tools->setTransformWarpStyle(QStringLiteral("arc")));
    const int base = view->history_count();
    tools->commitFreeTransform();
    QCOMPARE(view->history_count(), base + 1);
    QVERIFY(coverageBytes(view) != before);
}

void TransformSelectionTest::ctrlTStillLiftsPixels()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginDocument(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));
    QVERIFY(pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    QVERIFY(tools->beginFreeTransform(layer));
    QVERIFY(view->transform_session_active());
    QVERIFY(!pictura::transform_session_is_selection(*view));
    QVERIFY(tools->setTransformWidth(150.0, true));
    tools->commitFreeTransform();
    QVERIFY(!view->has_selection());
}

void TransformSelectionTest::borderTransformKeepsTheSelection()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));
    QVERIFY(tools->beginTransformSelection());
    QVERIFY(tools->setTransformWidth(140.0, true));
    tools->commitFreeTransform();
    QVERIFY(view->has_selection());
}

void TransformSelectionTest::selectMenuLockedWhilePending()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));
    QVERIFY(tools->beginTransformSelection());
    QAction* deselect = action(pictura::command_ids::SelectDeselect);
    QAction* inverse = action(pictura::command_ids::SelectInverse);
    QAction* feather = action(pictura::command_ids::SelectModifyFeather);
    QVERIFY(deselect && inverse && feather);
    QVERIFY(!deselect->isEnabled());
    QVERIFY(!inverse->isEnabled());
    QVERIFY(!feather->isEnabled());
    tools->cancelFreeTransform();
}

void TransformSelectionTest::toolSwitchPromptsForTheBorder()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));

    // Apply: the border commits, one state, the session ends.
    {
        QVERIFY(tools->beginTransformSelection());
        QVERIFY(tools->setTransformWidth(150.0, true));
        const int history = view->history_count();
        int asked = 0;
        tools->setTransformDecisionPrompt([&asked]() {
            ++asked;
            return pictura::TransformDecision::Apply;
        });
        tools->setActiveTool(pictura::ToolId::Brush);
        QCOMPARE(asked, 1);
        QVERIFY(!view->transform_session_active());
        QCOMPARE(view->history_count(), history + 1);
        tools->setTransformDecisionPrompt(nullptr);
        tools->setActiveTool(pictura::ToolId::Move);
    }

    // Don't Apply: the session ends with the mask and history unchanged.
    {
        const QByteArray before = coverageBytes(view);
        QVERIFY(tools->beginTransformSelection());
        QVERIFY(tools->setTransformWidth(150.0, true));
        const int history = view->history_count();
        int asked = 0;
        tools->setTransformDecisionPrompt([&asked]() {
            ++asked;
            return pictura::TransformDecision::DontApply;
        });
        tools->setActiveTool(pictura::ToolId::Brush);
        QCOMPARE(asked, 1);
        QVERIFY(!view->transform_session_active());
        QCOMPARE(view->history_count(), history);
        QCOMPARE(coverageBytes(view), before);
        tools->setTransformDecisionPrompt(nullptr);
        tools->setActiveTool(pictura::ToolId::Move);
    }

    // Cancel: the session stays and the tool switch is refused.
    {
        QVERIFY(tools->beginTransformSelection());
        QVERIFY(tools->setTransformWidth(150.0, true));
        int asked = 0;
        tools->setTransformDecisionPrompt([&asked]() {
            ++asked;
            return pictura::TransformDecision::Cancel;
        });
        tools->setActiveTool(pictura::ToolId::Brush);
        QCOMPARE(asked, 1);
        QVERIFY(view->transform_session_active());
        QCOMPARE(tools->activeTool(), pictura::ToolId::Move);
        tools->setTransformDecisionPrompt(nullptr);
        tools->cancelFreeTransform();
    }
}

void TransformSelectionTest::fieldsSeedFromTheSelectionBounds()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));
    QVERIFY(tools->beginTransformSelection());
    auto* x = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsTransformX"));
    auto* y = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsTransformY"));
    auto* w = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsTransformW"));
    auto* h = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsTransformH"));
    auto* angle = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsTransformAngle"));
    QVERIFY(x && y && w && h && angle);
    QVERIFY(std::abs(x->value() - (kBlock.x() + kBlock.width() / 2.0)) < kTolerance);
    QVERIFY(std::abs(y->value() - (kBlock.y() + kBlock.height() / 2.0)) < kTolerance);
    QVERIFY(std::abs(w->value() - 100.0) < kTolerance);
    QVERIFY(std::abs(h->value() - 100.0) < kTolerance);
    QVERIFY(std::abs(angle->value()) < kTolerance);
    tools->cancelFreeTransform();
}

void TransformSelectionTest::applyRecordsTransformSelection()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));
    QVERIFY(tools->beginTransformSelection());
    const int base = view->history_count();
    QVERIFY(tools->setTransformWidth(150.0, true));
    auto* apply = frame.findChild<QToolButton*>(QStringLiteral("optionsTransformApply"));
    QVERIFY(apply);
    apply->click();
    QVERIFY(!view->transform_session_active());
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Transform Selection"));
}

void TransformSelectionTest::againRepeatsABorderTransform()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));
    const pictura::selection_api::SelectionBounds start =
        pictura::selection_api::bounds(*view);
    QVERIFY(start.valid);
    QVERIFY(tools->beginTransformSelection());
    QVERIFY(tools->setTransformAngle(90.0));
    tools->commitFreeTransform();
    const int base = view->history_count();
    QVERIFY(view->has_selection());
    QAction* again = action(pictura::command_ids::EditTransformAgain);
    QVERIFY(again);
    QVERIFY(again->isEnabled());
    again->trigger();
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Transform Again"));
    QVERIFY(view->has_selection());
    const pictura::selection_api::SelectionBounds after =
        pictura::selection_api::bounds(*view);
    QVERIFY(after.valid);
    QVERIFY(std::abs(after.width - start.width) <= 1);
    QVERIFY(std::abs(after.height - start.height) <= 1);
}

void TransformSelectionTest::warpKeepsAnOpenBorderSession()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(!beginDocument(frame).isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(selectBlock(view));
    QVERIFY(tools->beginTransformSelection());
    QVERIFY(tools->setTransformWidth(150.0, true));
    QAction* warp = action(pictura::command_ids::EditTransformWarp);
    QVERIFY(warp);
    QVERIFY(warp->isEnabled());
    warp->trigger();
    QVERIFY(view->transform_session_active());
    QVERIFY(pictura::transform_warp_active(*view));
    QVERIFY(std::abs(pictura::transform_width_pct(*view) - 150.0) < 0.01);
    tools->cancelFreeTransform();
}

QTEST_MAIN(TransformSelectionTest)
#include "tst_transform_selection.moc"
