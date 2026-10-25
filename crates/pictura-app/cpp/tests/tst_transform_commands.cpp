// Edit > Transform beyond Free Transform (#529): the constrained Scale/Rotate
// sessions, the exact rotations and flips, and Transform Again.

#include <QtTest/QtTest>

#include "api/transform_api.h"
#include "commands.h"
#include "frame.h"
#include "image_view.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_transform/bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include "qt_test_support.h"

#include <QtGui/QAction>
#include <QtGui/QImage>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>

namespace {

constexpr int kRotateHandle = 8;
constexpr double kTolerance = 1e-6;

QRect layerRectOf(pictura::PictureView* view, const QString& path)
{
    const pictura::transform_api::LayerRect rect = pictura::transform_api::layerRect(*view, path);
    if (!rect.valid) {
        return QRect();
    }
    return QRect(rect.left, rect.top, rect.width(), rect.height());
}

bool isRed(const QColor& colour)
{
    return colour.red() > 200 && colour.green() < 80 && colour.blue() < 80;
}

bool samePoint(const QPointF& a, const QPointF& b)
{
    return std::abs(a.x() - b.x()) < kTolerance && std::abs(a.y() - b.y()) < kTolerance;
}

} // namespace

class TransformCommandsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void menuListsTheTransformCommands();
    void commandsEnableOnATransformableTarget();
    void constrainedModesDisableWhereTheyCannotSwitch();
    void scaleScalesWithoutRotating();
    void rotateRotatesWithoutScaling();
    void choosingAModeMidSessionKeepsTheBox();
    void rotate90ClockwiseSwapsTheBox();
    void exactRotationsAndFlipsMoveTheContent();
    void selectionTransformsOnlyItsPixels();
    void openSessionUpdatesInPlace();
    void undoCancelsAndClearsTheTransform();
    void againRepeatsTheLastTransform();
    void againUsesTheSourcePivot();
    void againIsUnavailableBeforeATransform();

private:
    // A new document with one transparent layer above the Background. `marks`
    // are filled black; `redMarks` are filled red after them.
    QString beginLayer(pictura::PicturaMainWindow& frame, const QList<QRect>& marks,
                       const QList<QRect>& redMarks = {});
    QAction* action(const char* id) const;

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void TransformCommandsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void TransformCommandsTest::cleanup()
{
    if (window_) {
        if (pictura::PictureView* view = window_->activeView();
            view && view->transform_session_active()) {
            view->cancel_transform();
        }
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
}

QString TransformCommandsTest::beginLayer(pictura::PicturaMainWindow& frame,
                                          const QList<QRect>& marks,
                                          const QList<QRect>& redMarks)
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
    for (const QRect& mark : marks) {
        if (!view->select_rect(mark.x(), mark.y(), mark.width(), mark.height(),
                               QStringLiteral("new"), 0.0)
            || !pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                                   false)) {
            return QString();
        }
    }
    for (const QRect& mark : redMarks) {
        if (!view->select_rect(mark.x(), mark.y(), mark.width(), mark.height(),
                               QStringLiteral("new"), 0.0)
            || !pictura::edit_fill(*view, qRgba(255, 0, 0, 255), -1, QStringLiteral("normal"),
                                   100, false)) {
            return QString();
        }
    }
    view->deselect();
    return layer;
}

QAction* TransformCommandsTest::action(const char* id) const
{
    return window_->registry()->action(QString::fromLatin1(id));
}

void TransformCommandsTest::menuListsTheTransformCommands()
{
    QMenu* edit = nullptr;
    for (QAction* top : window_->menuBar()->actions()) {
        if (top->text().remove(QLatin1Char('&')) == QStringLiteral("Edit")) {
            edit = top->menu();
        }
    }
    QVERIFY(edit != nullptr);

    QMenu* transform = nullptr;
    for (QAction* item : edit->actions()) {
        if (item->text() == QStringLiteral("Transform")) {
            transform = item->menu();
        }
    }
    QVERIFY(transform != nullptr);

    QStringList items;
    QAction* again = nullptr;
    for (QAction* item : transform->actions()) {
        if (item->isSeparator()) {
            items << QStringLiteral("<sep>");
            continue;
        }
        items << item->text();
        if (item->text() == QStringLiteral("Again")) {
            again = item;
        }
    }
    const QStringList expected = {
        QStringLiteral("Again"), QStringLiteral("<sep>"), QStringLiteral("Scale"),
        QStringLiteral("Rotate"), QStringLiteral("Skew"), QStringLiteral("Distort"),
        QStringLiteral("Perspective"), QStringLiteral("Warp"), QStringLiteral("<sep>"),
        QStringLiteral("Rotate 180°"), QStringLiteral("Rotate 90° Clockwise"),
        QStringLiteral("Rotate 90° Counter Clockwise"), QStringLiteral("<sep>"),
        QStringLiteral("Flip Horizontal"), QStringLiteral("Flip Vertical")};
    QCOMPARE(items, expected);
    QVERIFY(again != nullptr);
    QCOMPARE(again->shortcut(), QKeySequence(QStringLiteral("Shift+Ctrl+T")));
}

void TransformCommandsTest::commandsEnableOnATransformableTarget()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame, {QRect(10, 8, 12, 10)});
    QVERIFY(!layer.isEmpty());
    frame.registry()->refresh();

    const char* commands[] = {
        pictura::command_ids::EditTransformScale,
        pictura::command_ids::EditTransformRotate,
        pictura::command_ids::EditTransformRotate180,
        pictura::command_ids::EditTransformRotate90Cw,
        pictura::command_ids::EditTransformRotate90Ccw,
        pictura::command_ids::EditTransformFlipHorizontal,
        pictura::command_ids::EditTransformFlipVertical,
    };
    for (const char* id : commands) {
        QVERIFY2(action(id) != nullptr, id);
        QVERIFY2(action(id)->isEnabled(), id);
    }

    // The base layer is path "0"; make it a true Background, then select it.
    QVERIFY(frame.activeView()->background_from_layer(QStringLiteral("0")));
    frame.selectLayerPath(QStringLiteral("0"));
    frame.registry()->refresh();
    for (const char* id : commands) {
        QVERIFY2(!action(id)->isEnabled(), id);
    }
}

void TransformCommandsTest::constrainedModesDisableWhereTheyCannotSwitch()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame, {QRect(10, 8, 12, 10)});
    QVERIFY(!layer.isEmpty());
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(tools);

    // A similarity session accepts the switch.
    QVERIFY(tools->beginFreeTransform(layer));
    frame.registry()->refresh();
    QVERIFY(action(pictura::command_ids::EditTransformScale)->isEnabled());
    QVERIFY(action(pictura::command_ids::EditTransformRotate)->isEnabled());
    tools->cancelFreeTransform();

    // A projective session does not.
    QVERIFY(tools->beginTransformMode(layer, QStringLiteral("skew")));
    frame.registry()->refresh();
    QVERIFY(!action(pictura::command_ids::EditTransformScale)->isEnabled());
    QVERIFY(!action(pictura::command_ids::EditTransformRotate)->isEnabled());
    tools->cancelFreeTransform();

    // Neither does Warp mode.
    QVERIFY(tools->beginWarp(layer));
    frame.registry()->refresh();
    QVERIFY(!action(pictura::command_ids::EditTransformScale)->isEnabled());
    QVERIFY(!action(pictura::command_ids::EditTransformRotate)->isEnabled());
    tools->cancelFreeTransform();
}

void TransformCommandsTest::scaleScalesWithoutRotating()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame, {QRect(10, 8, 12, 10)});
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);

    QVERIFY(frame.registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::EditTransformScale)));
    QVERIFY(view->transform_session_active());
    QCOMPARE(pictura::transform_session_mode(*view), QStringLiteral("scale"));

    // The rotate band is inert in Scale mode.
    QCOMPARE(view->transform_hit_test(22.0, -10.0, 1.0), -1);
    QCOMPARE(view->transform_press(22.0, 8.0, 1.0, false, false), 1);
    QVERIFY(view->transform_move(28.0, 8.0, 1.0, false, false));

    const QPolygonF quad = pictura::transform_api::quad(*view).corners;
    QCOMPARE(quad.size(), 4);
    QVERIFY(std::abs(quad.at(1).x() - 28.0) < kTolerance);
    QVERIFY(std::abs(quad.at(1).y() - 8.0) < kTolerance);
    QVERIFY(std::abs(quad.at(3).x() - 10.0) < kTolerance);
    QVERIFY(std::abs(quad.at(3).y() - 18.0) < kTolerance);
    QVERIFY(std::abs(pictura::transform_angle_degrees(*view)) < kTolerance);
    tools->cancelFreeTransform();
}

void TransformCommandsTest::rotateRotatesWithoutScaling()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame, {QRect(10, 8, 12, 10)});
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);

    QVERIFY(frame.registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::EditTransformRotate)));
    QVERIFY(view->transform_session_active());
    QCOMPARE(pictura::transform_session_mode(*view), QStringLiteral("rotate"));

    // Every handle is a rotation handle in Rotate mode.
    QCOMPARE(view->transform_hit_test(22.0, 8.0, 1.0), kRotateHandle);
    QCOMPARE(view->transform_press(22.0, 8.0, 1.0, false, false), kRotateHandle);
    QVERIFY(view->transform_move(21.0, 19.0, 1.0, false, false));
    QVERIFY(std::abs(pictura::transform_angle_degrees(*view) - 90.0) < 1e-6);
    QVERIFY(std::abs(view->transform_scale_x() - 1.0) < kTolerance);
    QVERIFY(std::abs(view->transform_scale_y() - 1.0) < kTolerance);
    tools->cancelFreeTransform();
}

void TransformCommandsTest::choosingAModeMidSessionKeepsTheBox()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame, {QRect(10, 8, 12, 10)});
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);

    QVERIFY(tools->beginFreeTransform(layer));
    QVERIFY(tools->setTransformWidth(150.0, true));
    const QPolygonF before = pictura::transform_api::quad(*view).corners;
    QVERIFY(frame.registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::EditTransformScale)));
    QVERIFY(view->transform_session_active());
    QCOMPARE(pictura::transform_session_mode(*view), QStringLiteral("scale"));
    const QPolygonF after = pictura::transform_api::quad(*view).corners;
    QCOMPARE(after.size(), before.size());
    for (int i = 0; i < after.size(); ++i) {
        QVERIFY(std::abs(after.at(i).x() - before.at(i).x()) < kTolerance);
        QVERIFY(std::abs(after.at(i).y() - before.at(i).y()) < kTolerance);
    }
    tools->cancelFreeTransform();
}

void TransformCommandsTest::rotate90ClockwiseSwapsTheBox()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame, {QRect(10, 8, 12, 10)}, {QRect(10, 8, 4, 4)});
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);

    const int base = view->history_count();
    QVERIFY(frame.registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::EditTransformRotate90Cw)));
    QVERIFY(!view->transform_session_active());
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Rotate 90° Clockwise"));

    const QRect rect = layerRectOf(view, layer);
    QCOMPARE(rect.width(), 10);
    QCOMPARE(rect.height(), 12);
    // The red mark was at the top-left; a quarter turn puts it top-right.
    QVERIFY2(isRed(view->image().pixelColor(19, 9)), "the red mark moved");
}

void TransformCommandsTest::exactRotationsAndFlipsMoveTheContent()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame, {QRect(10, 8, 12, 10)}, {QRect(10, 8, 4, 4)});
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);

    QVERIFY(frame.registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::EditTransformRotate180)));
    QVERIFY2(isRed(view->image().pixelColor(20, 16)), "180° puts the mark opposite");
    QVERIFY2(!isRed(view->image().pixelColor(12, 10)), "the old corner is no longer red");

    QVERIFY(frame.registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::EditTransformFlipHorizontal)));
    QVERIFY2(isRed(view->image().pixelColor(12, 16)), "a horizontal flip mirrors x");
    QVERIFY2(!isRed(view->image().pixelColor(20, 16)), "the previous x is no longer red");

    QVERIFY(frame.registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::EditTransformFlipVertical)));
    QVERIFY2(isRed(view->image().pixelColor(12, 10)), "a vertical flip mirrors y");
    QVERIFY2(!isRed(view->image().pixelColor(12, 16)), "the previous y is no longer red");
}

void TransformCommandsTest::selectionTransformsOnlyItsPixels()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame, {QRect(10, 8, 18, 10)},
                                     {QRect(10, 8, 4, 4), QRect(24, 8, 4, 4)});
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);

    // A 4×8 bar over the left mark; the right mark stays outside the selection.
    QVERIFY(view->select_rect(10, 8, 4, 8, QStringLiteral("new"), 0.0));
    QVERIFY(frame.registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::EditTransformRotate90Cw)));
    QVERIFY2(isRed(view->image().pixelColor(14, 12)), "the selected mark rotated");
    QVERIFY2(!isRed(view->image().pixelColor(11, 9)), "the old selected mark moved away");
    QVERIFY2(isRed(view->image().pixelColor(26, 10)), "the unselected mark stayed");
}

void TransformCommandsTest::openSessionUpdatesInPlace()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame, {QRect(10, 8, 12, 10)});
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);

    QVERIFY(tools->beginFreeTransform(layer));
    const int base = view->history_count();
    QVERIFY(frame.registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::EditTransformRotate90Cw)));
    QVERIFY(view->transform_session_active());
    QVERIFY(std::abs(pictura::transform_angle_degrees(*view) - 90.0) < 1e-6);
    QCOMPARE(view->history_count(), base);

    tools->commitFreeTransform();
    QVERIFY(!view->transform_session_active());
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Free Transform"));
    const QRect rect = layerRectOf(view, layer);
    QCOMPARE(rect.width(), 10);
    QCOMPARE(rect.height(), 12);
}

void TransformCommandsTest::againRepeatsTheLastTransform()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame, {QRect(10, 8, 12, 10)}, {QRect(10, 8, 4, 4)});
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);

    const int base = view->history_count();
    QVERIFY(frame.registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::EditTransformRotate90Cw)));
    QCOMPARE(view->history_label(base), QStringLiteral("Rotate 90° Clockwise"));
    QVERIFY2(isRed(view->image().pixelColor(19, 9)), "the first quarter turn");

    frame.registry()->refresh();
    QVERIFY(action(pictura::command_ids::EditTransformAgain)->isEnabled());
    QVERIFY(frame.registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::EditTransformAgain)));
    QCOMPARE(view->history_count(), base + 2);
    QCOMPARE(view->history_label(base + 1), QStringLiteral("Transform Again"));
    QVERIFY2(isRed(view->image().pixelColor(20, 16)), "Again adds another quarter turn");
}

void TransformCommandsTest::againUsesTheSourcePivot()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame, {QRect(5, 5, 30, 20)});
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);

    // Move the box, then put the pivot on its moved top-left and commit. The
    // recorded offset is in the source frame, so Again must keep the pivot at
    // the repeated box's top-left instead of adding the carried translation.
    QVERIFY(tools->beginFreeTransform(layer));
    QCOMPARE(view->transform_press(27.0, 22.0, 1.0, false, false), 9);
    QVERIFY(view->transform_move(32.0, 27.0, 1.0, false, false));
    QVERIFY(view->transform_release());
    QVERIFY(std::abs(pictura::transform_pivot_x(*view) - 25.0) < kTolerance);
    QVERIFY(std::abs(pictura::transform_pivot_y(*view) - 20.0) < kTolerance);
    QVERIFY(pictura::transform_set_pivot(*view, 10.0, 10.0));
    tools->commitFreeTransform();

    const pictura::transform_api::LayerRect committed =
        pictura::transform_api::layerRect(*view, layer);
    QVERIFY(committed.valid);
    QCOMPARE(committed.left, 10);
    QCOMPARE(committed.top, 10);

    QVERIFY(tools->beginFreeTransform(layer));
    QVERIFY(pictura::transform_repeat(*view));
    const QPolygonF quad = pictura::transform_api::quad(*view).corners;
    QCOMPARE(quad.size(), 4);
    const QPointF pivot(pictura::transform_pivot_x(*view), pictura::transform_pivot_y(*view));
    QVERIFY2(samePoint(pivot, quad.at(0)),
             qPrintable(QStringLiteral("pivot %1,%2 corner %3,%4")
                            .arg(pivot.x())
                            .arg(pivot.y())
                            .arg(quad.at(0).x())
                            .arg(quad.at(0).y())));
    tools->cancelFreeTransform();
}

void TransformCommandsTest::undoCancelsAndClearsTheTransform()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame, {QRect(5, 5, 30, 20)});
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* canvas = frame.findChild<pictura::ImageView*>();
    QVERIFY(view && tools && canvas);
    const int base = view->history_index();

    QVERIFY(tools->beginFreeTransform(layer));
    QVERIFY(tools->setTransformWidth(150.0, true));
    QVERIFY(!canvas->transformPreviewLayerForTest().isNull());

    QVERIFY(frame.registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::EditUndo)));
    QVERIFY(!view->transform_session_active());
    QVERIFY(canvas->transformPreviewLayerForTest().isNull());
    QVERIFY(pictura::transform_api::quad(*view).corners.isEmpty());
    QVERIFY(!tools->transformControlsSession());
    QCOMPARE(view->history_index(), base - 1);
}

void TransformCommandsTest::againIsUnavailableBeforeATransform()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame, {QRect(10, 8, 12, 10)});
    QVERIFY(!layer.isEmpty());
    frame.registry()->refresh();
    QVERIFY(!action(pictura::command_ids::EditTransformAgain)->isEnabled());
}

QTEST_MAIN(TransformCommandsTest)
#include "tst_transform_commands.moc"
