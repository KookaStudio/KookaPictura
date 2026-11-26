// The Pen tool group: Pen (#32), Freeform Pen (#33), Add Anchor Point (#34),
// Delete Anchor Point (#35), and Convert Point (#36).

#include <QtTest/QtTest>

#include "icons.h"
#include "panels/numeric_field.h"
#include "panels/paths_panel.h"
#include "selftest_paint_fixture.h"

#include "api/layers_bridge.h"

#include <QtGui/QKeyEvent>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

// The active tool's own options-bar control: each tool has its own page.
template <typename T>
T* visibleChild(pictura::PicturaMainWindow& frame, const QString& name)
{
    for (T* child : frame.findChildren<T*>(name)) {
        if (child->isVisible()) {
            return child;
        }
    }
    return nullptr;
}

int points(const Fixture& f, int sp) { return pictura::path_point_count(*f.view, sp); }

QList<double> point(const Fixture& f, int sp, int pt)
{
    QList<double> out;
    for (double v : pictura::path_point(*f.view, sp, pt)) {
        out.append(v);
    }
    return out;
}

void click(const Fixture& f, const QPointF& p, Qt::KeyboardModifiers mods = Qt::NoModifier)
{
    f.drag({p}, mods);
}

// A closed 40 x 40 square at (10, 10) drawn with the Pen: four states.
void drawSquare(const Fixture& f)
{
    for (const QPointF& p : {QPointF(10, 10), QPointF(50, 10), QPointF(50, 50), QPointF(10, 50),
                             QPointF(10, 10)}) {
        click(f, p);
    }
}

// The existing Pen checks exercise the Work Path, so they opt into Path mode.
void pathMode(const Fixture& f)
{
    auto* group = f.tools->optionsFor<pictura::PenGroup>(pictura::ToolId::Pen);
    pictura::PenOptions o = group->get();
    o.mode = 1;
    group->set(o);
}

} // namespace

class PenToolsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void penTool();
    void penModeBar();
    void penShapeMode();
    void penPathMode();
    void penPixelsMode();
    void penAutoAddDelete();
    void freeformPenTool();
    void anchorPointTools();
    void convertPointTool();
    void pathsPanelListsTheWorkPath();
    void ctrlUndoStepsBackAnchorEdits();
    void ctrlUndoCancelsALiveGesture();
    void ctrlUndoCancelsAConvertDrag();
    void ctrlUndoCancelsAFreeformTrail();
    void enterInFieldKeepsPathActive();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void PenToolsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void PenToolsTest::penTool()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_pen_seed"));
    QVERIFY2(f.ok(), "pen fixture");
    pathMode(f);

    // P and Shift+P cycle the Pen and Freeform Pen.
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    frame.activateWindow();
    QCoreApplication::processEvents();
    const auto sendKey = [&frame](int key, Qt::KeyboardModifiers mods, const QString& text) {
        QKeyEvent event(QEvent::KeyPress, key, mods, text);
        QApplication::sendEvent(&frame, &event);
    };
    frame.setActiveTool(pictura::ToolId::Move);
    sendKey(Qt::Key_P, Qt::NoModifier, QStringLiteral("p"));
    QCOMPARE(frame.activeTool(), pictura::ToolId::Pen);
    sendKey(Qt::Key_P, Qt::ShiftModifier, QStringLiteral("P"));
    QCOMPARE(frame.activeTool(), pictura::ToolId::FreeformPen);
    sendKey(Qt::Key_P, Qt::ShiftModifier, QStringLiteral("P"));
    QCOMPARE(frame.activeTool(), pictura::ToolId::Pen);
    // From an anchor tool, P goes back to a P tool.
    frame.setActiveTool(pictura::ToolId::ConvertPoint);
    sendKey(Qt::Key_P, Qt::NoModifier, QStringLiteral("p"));
    QCOMPARE(frame.activeTool(), pictura::ToolId::Pen);

    auto* autoAdd = frame.findChild<QCheckBox*>(QStringLiteral("optionsPenAutoAddDelete"));
    auto* rubber = frame.findChild<QCheckBox*>(QStringLiteral("optionsPenRubberBand"));
    QVERIFY2(autoAdd && rubber, "pen bar");
    QVERIFY2(autoAdd->isChecked() && !rubber->isChecked(), "pen bar defaults");

    // Click, click, drag: two corners and a smooth point with mirrored handles.
    const int base = f.view->history_index();
    click(f, QPointF(10, 10));
    QVERIFY2(f.committedOnce(base, "New Work Path"), "first anchor");
    click(f, QPointF(50, 10));
    QVERIFY2(f.committedOnce(base + 1, "Add Anchor Point"), "second anchor");
    f.drag({QPointF(50, 50), QPointF(55, 50), QPointF(60, 50)});
    QVERIFY2(f.committedOnce(base + 2, "Add Anchor Point"), "a drag is one state");
    QCOMPARE(point(f, 0, 1).at(2), 0.0);
    const QList<double> smooth = point(f, 0, 2);
    QCOMPARE(smooth.at(8), 1.0);
    QCOMPARE(QPointF(smooth.at(6), smooth.at(7)), QPointF(60, 50));
    QCOMPARE(QPointF(smooth.at(3), smooth.at(4)), QPointF(40, 50));
    QCOMPARE(f.canvas->pathOverlayAnchorCountForTest(), 3);

    // The Rubber Band previews the next segment while the cursor hovers.
    rubber->setChecked(true);
    f.canvas->mouseMoved(QPointF(30, 70));
    QVERIFY2(f.canvas->pathOverlayHasPreviewForTest(), "no rubber band");
    rubber->setChecked(false);
    QVERIFY2(!f.canvas->pathOverlayHasPreviewForTest(), "rubber band left on screen");

    // Over the first anchor the Pen shows its close-path cursor.
    const auto cursorIs = [&f](const char* id) {
        return f.canvas->cursor().pixmap().toImage()
            == pictura::cursor(QString::fromLatin1(id), 2, 2).pixmap().toImage();
    };
    QVERIFY2(!pictura::cursor(QStringLiteral("tool.pen.close"), 2, 2).pixmap().isNull(),
             "close cursor asset");
    f.canvas->mouseMoved(QPointF(30, 30));
    QVERIFY2(cursorIs("tool.pen"), "plain pen cursor");
    f.canvas->mouseMoved(QPointF(11, 11));
    QVERIFY2(cursorIs("tool.pen.close"), "no close cursor over the first anchor");
    f.canvas->mouseMoved(QPointF(50, 11));
    QVERIFY2(cursorIs("tool.pen"), "close cursor away from the first anchor");
    f.canvas->mouseMoved(QPointF(11, 11));

    // Clicking the first anchor closes the subpath, and the cursor goes back.
    click(f, QPointF(11, 11));
    QVERIFY2(f.committedOnce(base + 3, "Close Path"), "close");
    QVERIFY(pictura::path_subpath_closed(*f.view, 0));
    QCOMPARE(pictura::path_editing_subpath(*f.view), -1);
    QVERIFY2(cursorIs("tool.pen"), "close cursor after closing");

    // Shift snaps the next anchor to 45° from the previous one; Enter ends it.
    click(f, QPointF(70, 80));
    click(f, QPointF(90, 83), Qt::ShiftModifier);
    QCOMPARE(point(f, 1, 1).at(1), 80.0);
    QCOMPARE(pictura::path_editing_subpath(*f.view), 1);
    sendKey(Qt::Key_Return, Qt::NoModifier, QString());
    QCOMPARE(pictura::path_editing_subpath(*f.view), -1);
    QVERIFY(!pictura::path_subpath_closed(*f.view, 1));

    // Clicking an open endpoint resumes drawing from it.
    click(f, QPointF(90, 80));
    QCOMPARE(pictura::path_editing_subpath(*f.view), 1);
    click(f, QPointF(90, 95));
    QCOMPARE(points(f, 1), 3);
    // Switching tools leaves it open; undo takes the last anchor back.
    frame.setActiveTool(pictura::ToolId::Move);
    QCOMPARE(pictura::path_editing_subpath(*f.view), -1);
    QCOMPARE(f.canvas->pathOverlayAnchorCountForTest(), 0);
    QVERIFY(f.view->undo());
    QCOMPARE(points(f, 1), 2);
}

void PenToolsTest::penModeBar()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_pen_mode_bar"));
    QVERIFY2(f.ok(), "pen mode fixture");
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    f.tools->optionsFor<pictura::PenGroup>(pictura::ToolId::Pen)->set({});
    frame.setActiveTool(pictura::ToolId::Pen);

    auto* mode = visibleChild<QComboBox>(frame, QStringLiteral("optionsPenMode"));
    QVERIFY2(mode, "pen mode bar");
    QCOMPARE(mode->count(), 3);
    QCOMPARE(mode->itemText(0), QStringLiteral("Shape"));
    QCOMPARE(mode->itemText(1), QStringLiteral("Path"));
    QCOMPARE(mode->itemText(2), QStringLiteral("Pixels"));
    QCOMPARE(mode->currentIndex(), 0);

    // The two Pen pages share the mode.
    mode->setCurrentIndex(1);
    QCOMPARE(f.tools->optionsFor<pictura::PenGroup>(pictura::ToolId::Pen)->get().mode, 1);
    frame.setActiveTool(pictura::ToolId::FreeformPen);
    auto* freeform = visibleChild<QComboBox>(frame, QStringLiteral("optionsPenMode"));
    QVERIFY2(freeform, "freeform mode bar");
    QCOMPARE(freeform->currentIndex(), 1);
    freeform->setCurrentIndex(2);
    QCOMPARE(f.tools->optionsFor<pictura::PenGroup>(pictura::ToolId::Pen)->get().mode, 2);
    QCOMPARE(mode->currentIndex(), 2);
}

void PenToolsTest::penShapeMode()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_pen_shape_mode"));
    QVERIFY2(f.ok(), "pen shape fixture");
    auto* group = f.tools->optionsFor<pictura::PenGroup>(pictura::ToolId::Pen);
    pictura::PenOptions options = group->get();
    options.mode = 0;
    group->set(options);
    frame.setActiveTool(pictura::ToolId::Pen);

    const int layers = f.view->layer_count();
    drawSquare(f);
    QCOMPARE(f.view->layer_count(), layers + 1);
    QVERIFY2(!pictura::paths_has_work_path(*f.view), "shape left a Work Path");
    QCOMPARE(f.view->history_label(f.view->history_index()), QStringLiteral("Pen Tool"));
    QVERIFY(f.view->sample_argb(30, 30) != QColor(Qt::white).rgba());
}

void PenToolsTest::penPathMode()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_pen_path_mode"));
    QVERIFY2(f.ok(), "pen path fixture");
    auto* group = f.tools->optionsFor<pictura::PenGroup>(pictura::ToolId::Pen);
    pictura::PenOptions options = group->get();
    options.mode = 1;
    group->set(options);
    frame.setActiveTool(pictura::ToolId::Pen);

    const int saved = pictura::paths_saved_count(*f.view);
    drawSquare(f);
    QCOMPARE(pictura::paths_saved_count(*f.view), saved + 1);
    QCOMPARE(pictura::paths_saved_name(*f.view, saved), QStringLiteral("Path 1"));
    QVERIFY(pictura::paths_has_work_path(*f.view));
    QCOMPARE(pictura::path_subpath_count(*f.view), 1);
    QCOMPARE(pictura::path_point_count(*f.view, 0), 4);
}

void PenToolsTest::penPixelsMode()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_pen_pixels_mode"));
    QVERIFY2(f.ok(), "pen pixels fixture");
    auto* group = f.tools->optionsFor<pictura::PenGroup>(pictura::ToolId::Pen);
    pictura::PenOptions options = group->get();
    options.mode = 2;
    group->set(options);
    frame.setActiveTool(pictura::ToolId::Pen);

    const int layers = f.view->layer_count();
    drawSquare(f);
    QCOMPARE(f.view->layer_count(), layers);
    QCOMPARE(f.view->history_label(f.view->history_index()), QStringLiteral("Stroke Path"));
    QVERIFY(f.view->sample_argb(10, 10) != QColor(Qt::white).rgba());
}

void PenToolsTest::penAutoAddDelete()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_pen_auto_seed"));
    QVERIFY2(f.ok(), "pen fixture");
    pathMode(f);
    frame.setActiveTool(pictura::ToolId::Pen);
    drawSquare(f);
    QCOMPARE(points(f, 0), 4);

    // Over a segment the Pen adds an anchor; over an anchor it deletes one.
    const int base = f.view->history_index();
    click(f, QPointF(30, 10));
    QVERIFY2(f.committedOnce(base, "Add Anchor Point"), "auto add");
    QCOMPARE(points(f, 0), 5);
    click(f, QPointF(30, 10));
    QVERIFY2(f.committedOnce(base + 1, "Delete Anchor Point"), "auto delete");
    QCOMPARE(points(f, 0), 4);

    // Off, the same click starts a new subpath.
    f.tools->optionsFor<pictura::PenGroup>(pictura::ToolId::Pen)->set({false, false, 2.0, false, 1});
    click(f, QPointF(30, 10));
    QCOMPARE(pictura::path_subpath_count(*f.view), 2);
    f.tools->optionsFor<pictura::PenGroup>(pictura::ToolId::Pen)->set({});
}

void PenToolsTest::freeformPenTool()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_freeform_seed"));
    QVERIFY2(f.ok(), "freeform fixture");
    pathMode(f);
    frame.setActiveTool(pictura::ToolId::FreeformPen);
    auto* fit = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsFreeformPenCurveFit"));
    QVERIFY2(fit && fit->value() == 2.0, "freeform bar");

    // A wobbly square loop that ends near its start.
    QList<QPointF> loop;
    for (int i = 0; i <= 40; ++i) {
        loop.append(QPointF(10 + i, 10 + (i % 2) * 0.4));
    }
    for (int i = 0; i <= 40; ++i) {
        loop.append(QPointF(50, 10 + i));
    }
    for (int i = 0; i <= 40; ++i) {
        loop.append(QPointF(50 - i, 50));
    }
    for (int i = 0; i <= 39; ++i) {
        loop.append(QPointF(10, 50 - i));
    }
    loop.append(QPointF(10, 10.5));
    const int base = f.view->history_index();
    f.drag(loop);
    QVERIFY2(f.committedOnce(base, "Freeform Pen"), "one Freeform Pen state");
    QVERIFY(pictura::path_subpath_closed(*f.view, 0));
    QVERIFY2(points(f, 0) >= 4 && points(f, 0) <= 6, "the loop was not simplified");

    // A higher Curve Fit keeps fewer anchors of the same wavy stroke.
    QList<QPointF> wave;
    for (int i = 0; i <= 80; ++i) {
        wave.append(QPointF(10 + i, 80 + 3 * std::sin(i / 4.0)));
    }
    f.tools->optionsFor<pictura::PenGroup>(pictura::ToolId::Pen)->set({true, false, 0.5, false, 1});
    f.drag(wave);
    f.tools->optionsFor<pictura::PenGroup>(pictura::ToolId::Pen)->set({true, false, 10.0, false, 1});
    f.drag(wave);
    QVERIFY2(!pictura::path_subpath_closed(*f.view, 1), "an open stroke closed");
    QVERIFY2(points(f, 1) > points(f, 2), "Curve Fit did not simplify");
    QCOMPARE(points(f, 2), 2);
    f.tools->optionsFor<pictura::PenGroup>(pictura::ToolId::Pen)->set({});

    // A click draws nothing.
    const int clicked = f.view->history_index();
    click(f, QPointF(70, 20));
    QCOMPARE(f.view->history_index(), clicked);
}

void PenToolsTest::anchorPointTools()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_anchor_seed"));
    QVERIFY2(f.ok(), "anchor fixture");
    pathMode(f);
    frame.setActiveTool(pictura::ToolId::Pen);
    drawSquare(f);

    frame.setActiveTool(pictura::ToolId::AddAnchorPoint);
    QCOMPARE(f.canvas->pathOverlayAnchorCountForTest(), 4);
    int base = f.view->history_index();
    click(f, QPointF(50, 30));
    QVERIFY2(f.committedOnce(base, "Add Anchor Point"), "add");
    const QList<double> added = point(f, 0, 2);
    QVERIFY2(std::abs(added.at(0) - 50) < 1e-6 && std::abs(added.at(1) - 30) < 0.1,
             "the anchor landed off the click");
    click(f, QPointF(80, 80));
    QCOMPARE(f.view->history_index(), base + 1);

    frame.setActiveTool(pictura::ToolId::DeleteAnchorPoint);
    base = f.view->history_index();
    click(f, QPointF(49, 31));
    QVERIFY2(f.committedOnce(base, "Delete Anchor Point"), "delete");
    QCOMPARE(points(f, 0), 4);
    click(f, QPointF(30, 30));
    QCOMPARE(f.view->history_index(), base + 1);
    for (const QPointF& p : {QPointF(10, 10), QPointF(50, 10), QPointF(50, 50), QPointF(10, 50)}) {
        click(f, p);
    }
    QCOMPARE(pictura::path_subpath_count(*f.view), 0);
}

void PenToolsTest::convertPointTool()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_convert_seed"));
    QVERIFY2(f.ok(), "convert fixture");
    pathMode(f);
    frame.setActiveTool(pictura::ToolId::Pen);
    drawSquare(f);
    frame.setActiveTool(pictura::ToolId::ConvertPoint);

    // Dragging a corner pulls out a smooth pair of handles.
    int base = f.view->history_index();
    f.drag({QPointF(50, 10), QPointF(55, 10), QPointF(60, 10)});
    QVERIFY2(f.committedOnce(base, "Convert Point"), "drag");
    QList<double> p = point(f, 0, 1);
    QCOMPARE(p.at(8), 1.0);
    QCOMPARE(QPointF(p.at(3), p.at(4)), QPointF(40, 10));

    // Dragging one handle breaks the pair.
    f.drag({QPointF(40, 10), QPointF(40, 0)});
    QVERIFY2(f.committedOnce(base + 1, "Convert Point"), "handle drag");
    p = point(f, 0, 1);
    QCOMPARE(p.at(8), 0.0);
    QCOMPARE(QPointF(p.at(6), p.at(7)), QPointF(60, 10));

    // A click makes it a corner again; on a corner it records nothing.
    click(f, QPointF(50, 10));
    QVERIFY2(f.committedOnce(base + 2, "Convert Point"), "click");
    p = point(f, 0, 1);
    QVERIFY(p.at(2) == 0.0 && p.at(5) == 0.0);
    click(f, QPointF(50, 10));
    QCOMPARE(f.view->history_index(), base + 3);
}

void PenToolsTest::pathsPanelListsTheWorkPath()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_paths_panel_seed"));
    QVERIFY2(f.ok(), "paths fixture");
    pathMode(f);
    auto* panel = frame.findChild<pictura::PathsPanel*>(QStringLiteral("pathsPanel"));
    QVERIFY2(panel, "paths panel");
    QListWidget* list = panel->listForTest();
    // The panels follow an edit through the frame's debounced refresh.
    QTRY_COMPARE(list->count(), 0);

    // The first anchor creates the Work Path row, selected, in italics.
    frame.setActiveTool(pictura::ToolId::Pen);
    click(f, QPointF(10, 10));
    QTRY_COMPARE(list->count(), 1);
    QCOMPARE(list->item(0)->text(), QStringLiteral("Work Path"));
    QVERIFY(list->item(0)->font().italic());
    QVERIFY(list->item(0)->isSelected());
    QVERIFY(!list->item(0)->icon().isNull());
    click(f, QPointF(60, 60));
    // negative window: proves another anchor adds no second Work Path row
    QTest::qWait(300);
    QCOMPARE(list->count(), 1);

    // Undoing every anchor takes the row away again.
    QVERIFY(f.view->undo() && f.view->undo());
    QTRY_COMPARE(list->count(), 0);
}

// Ctrl+Z while a Pen tool is active walks back one committed edit per press.
void PenToolsTest::ctrlUndoStepsBackAnchorEdits()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_pen_undo_seed"));
    QVERIFY2(f.ok(), "pen undo fixture");
    pathMode(f);
    frame.show();
    QVERIFY(QTest::qWaitForWindowActive(&frame));
    frame.setActiveTool(pictura::ToolId::Pen);
    click(f, QPointF(10, 10));
    click(f, QPointF(50, 10));
    click(f, QPointF(50, 50));
    QCOMPARE(points(f, 0), 3);
    const int base = f.view->history_index();
    f.canvas->setFocus();
    QTest::keyClick(f.canvas, Qt::Key_Z, Qt::ControlModifier);
    QCOMPARE(points(f, 0), 2);
    QCOMPARE(f.view->history_index(), base - 1);
    QTest::keyClick(f.canvas, Qt::Key_Z, Qt::ControlModifier);
    QCOMPARE(points(f, 0), 1);
    QCOMPARE(f.view->history_index(), base - 2);
    QTest::keyClick(f.canvas, Qt::Key_Z, Qt::ControlModifier);
    QCOMPARE(pictura::path_subpath_count(*f.view), 0);
    QCOMPARE(f.view->history_index(), base - 3);
}

// A live gesture is one session step: Ctrl+Z cancels it — restoring the path
// to its pre-press state — and the release records nothing. The next Ctrl+Z
// then steps back the previous committed edit.
void PenToolsTest::ctrlUndoCancelsALiveGesture()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_pen_gesture_seed"));
    QVERIFY2(f.ok(), "pen gesture fixture");
    pathMode(f);
    frame.show();
    QVERIFY(QTest::qWaitForWindowActive(&frame));
    frame.setActiveTool(pictura::ToolId::Pen);
    f.canvas->setFocus();

    // First anchor, cancelled before the release: the path stays empty and no
    // state is recorded.
    f.canvas->mousePressed(QPointF(10, 10), Qt::LeftButton, 0);
    f.canvas->mouseMoved(QPointF(20, 10));
    QCOMPARE(points(f, 0), 1);
    QTest::keyClick(f.canvas, Qt::Key_Z, Qt::ControlModifier);
    QCOMPARE(points(f, 0), 0);
    f.canvas->mouseReleased(QPointF(20, 10));
    QCOMPARE(f.view->history_count(), 1);

    // A committed anchor, then a live drag cancelled: the committed anchor
    // survives, the live one is gone, and the release records nothing.
    click(f, QPointF(10, 10));
    const int base = f.view->history_index();
    f.canvas->mousePressed(QPointF(50, 10), Qt::LeftButton, 0);
    f.canvas->mouseMoved(QPointF(60, 10));
    QCOMPARE(points(f, 0), 2);
    QTest::keyClick(f.canvas, Qt::Key_Z, Qt::ControlModifier);
    QCOMPARE(points(f, 0), 1);
    QCOMPARE(f.view->history_index(), base);
    f.canvas->mouseReleased(QPointF(60, 10));
    QCOMPARE(f.view->history_index(), base);
    // The gesture was consumed by the session: the next Ctrl+Z is the document.
    QTest::keyClick(f.canvas, Qt::Key_Z, Qt::ControlModifier);
    QCOMPARE(points(f, 0), 0);
}

// A Convert Point handle drag is cancelled the same way: the handles go back
// and the release records nothing.
void PenToolsTest::ctrlUndoCancelsAConvertDrag()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_convert_undo_seed"));
    QVERIFY2(f.ok(), "convert undo fixture");
    pathMode(f);
    frame.show();
    QVERIFY(QTest::qWaitForWindowActive(&frame));
    frame.setActiveTool(pictura::ToolId::Pen);
    drawSquare(f);
    frame.setActiveTool(pictura::ToolId::ConvertPoint);
    f.canvas->setFocus();

    const int base = f.view->history_index();
    f.canvas->mousePressed(QPointF(50, 10), Qt::LeftButton, 0);
    f.canvas->mouseMoved(QPointF(60, 10));
    QCOMPARE(point(f, 0, 1).at(8), 1.0);
    QTest::keyClick(f.canvas, Qt::Key_Z, Qt::ControlModifier);
    QCOMPARE(point(f, 0, 1).at(8), 0.0);
    f.canvas->mouseReleased(QPointF(60, 10));
    QCOMPARE(f.view->history_index(), base);
}

// A Freeform Pen trail lives only in the handler until the release; Ctrl+Z
// clears it and the release records nothing.
void PenToolsTest::ctrlUndoCancelsAFreeformTrail()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_freeform_undo_seed"));
    QVERIFY2(f.ok(), "freeform undo fixture");
    pathMode(f);
    frame.show();
    QVERIFY(QTest::qWaitForWindowActive(&frame));
    frame.setActiveTool(pictura::ToolId::FreeformPen);
    f.canvas->setFocus();

    f.canvas->mousePressed(QPointF(10, 10), Qt::LeftButton, 0);
    for (int i = 0; i <= 40; ++i) {
        f.canvas->mouseMoved(QPointF(10 + i, 20));
    }
    QVERIFY2(f.canvas->pathOverlayHasPreviewForTest(), "trail preview");
    QTest::keyClick(f.canvas, Qt::Key_Z, Qt::ControlModifier);
    QVERIFY2(!f.canvas->pathOverlayHasPreviewForTest(), "trail not cleared");
    f.canvas->mouseReleased(QPointF(50, 20));
    QCOMPARE(f.view->history_count(), 1);
    QCOMPARE(pictura::path_subpath_count(*f.view), 0);
}

void PenToolsTest::enterInFieldKeepsPathActive()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_pen_enter"));
    QVERIFY2(f.ok(), "pen enter fixture");
    pathMode(f);
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    frame.activateWindow();
    QCoreApplication::processEvents();

    frame.setActiveTool(pictura::ToolId::Pen);
    click(f, QPointF(10, 10));
    click(f, QPointF(50, 10));
    QVERIFY(pictura::path_editing_subpath(*f.view) >= 0);

    // Enter in a value field commits it and leaves the subpath open.
    auto* zoom = frame.findChild<pictura::NumericField*>(QStringLiteral("statusZoomField"));
    QVERIFY(zoom);
    auto* edit = zoom->findChild<QLineEdit*>();
    QVERIFY(edit);
    edit->setFocus();
    QCoreApplication::processEvents();
    QVERIFY(edit->hasFocus());
    edit->selectAll();
    QTest::keyClicks(edit, QStringLiteral("200"));
    QTest::keyClick(edit, Qt::Key_Return);
    QCOMPARE(zoom->value(), 200.0);
    QVERIFY(!edit->hasFocus());
    QVERIFY(pictura::path_editing_subpath(*f.view) >= 0);

    // Enter with no value input focused ends the subpath open.
    f.canvas->setFocus();
    QCoreApplication::processEvents();
    QKeyEvent enter(QEvent::KeyPress, Qt::Key_Return, Qt::NoModifier);
    QApplication::sendEvent(&frame, &enter);
    QCOMPARE(pictura::path_editing_subpath(*f.view), -1);
}

PICTURA_QT_TEST_MAIN(PenToolsTest)
#include "tst_pen_tools.moc"
