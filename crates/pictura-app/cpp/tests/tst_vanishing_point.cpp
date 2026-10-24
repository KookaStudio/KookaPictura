#include <QtTest/QtTest>

#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>

#include <cmath>
#include <functional>
#include <memory>

#include "commands.h"
#include "filter_commands.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools/vanishing_point.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_mode.cxxqt.h"
#include "vp_canvas.h"
#include "vp_dialog.h"

#include "qt_test_support.h"

using pictura::VanishingPointDialog;
using pictura::VpCanvas;

namespace {

const QStringList kVpPath = {QStringLiteral("Filter"), QStringLiteral("Vanishing Point…")};

// A square-on plane inside the 400×300 test documents, and a second one
// overlapping its lower right.
const QList<QPointF> kSquare = {QPointF(100, 60), QPointF(300, 60), QPointF(300, 240),
                                QPointF(100, 240)};
const QList<QPointF> kOverlap = {QPointF(200, 120), QPointF(380, 120), QPointF(380, 280),
                                 QPointF(200, 280)};

VanishingPointDialog* openVpDialog()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<VanishingPointDialog*>(widget);
            dialog && dialog->isVisible()) {
            return dialog;
        }
    }
    return nullptr;
}

void sendMouse(QWidget* widget, QEvent::Type type, const QPointF& at, Qt::MouseButton button,
               Qt::MouseButtons buttons, Qt::KeyboardModifiers modifiers = Qt::NoModifier)
{
    QMouseEvent event(type, at, widget->mapToGlobal(at), button, buttons, modifiers);
    QApplication::sendEvent(widget, &event);
}

// A left drag across the canvas between document points.
void drag(VpCanvas* canvas, const QPointF& from, const QPointF& to,
          Qt::KeyboardModifiers modifiers = Qt::NoModifier)
{
    const QPointF a = canvas->toWidget(from);
    const QPointF b = canvas->toWidget(to);
    sendMouse(canvas, QEvent::MouseButtonPress, a, Qt::LeftButton, Qt::LeftButton, modifiers);
    for (int i = 1; i <= 5; ++i) {
        sendMouse(canvas, QEvent::MouseMove, a + (b - a) * (i / 5.0), Qt::NoButton, Qt::LeftButton,
                  modifiers);
    }
    sendMouse(canvas, QEvent::MouseButtonRelease, b, Qt::LeftButton, Qt::NoButton, modifiers);
}

void click(VpCanvas* canvas, const QPointF& document,
           Qt::KeyboardModifiers modifiers = Qt::NoModifier)
{
    const QPointF at = canvas->toWidget(document);
    sendMouse(canvas, QEvent::MouseButtonPress, at, Qt::LeftButton, Qt::LeftButton, modifiers);
    sendMouse(canvas, QEvent::MouseButtonRelease, at, Qt::LeftButton, Qt::NoButton, modifiers);
}

// Click four corners with Create Plane.
void makePlane(VanishingPointDialog* dialog, const QList<QPointF>& corners)
{
    dialog->selectTool(VanishingPointDialog::CreatePlane);
    for (const QPointF& p : corners) {
        click(dialog->canvas(), p);
    }
}

QPointF corner(int plane, int k)
{
    const rust::Vec<double> c = pictura::vp_plane_corners(plane);
    return c.size() == 8 ? QPointF(c[2 * k], c[2 * k + 1]) : QPointF(-1, -1);
}

bool near(const QPointF& a, const QPointF& b)
{
    return std::abs(a.x() - b.x()) < 0.05 && std::abs(a.y() - b.y()) < 0.05;
}

bool white(const QImage& image, int x, int y)
{
    return image.pixelColor(x, y) == QColor(Qt::white);
}

} // namespace

class VanishingPointTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void menuFollowsDepth();
    void opensWithControls();
    void fourClicksCreateAPlane();
    void dragReshapesAndMoves();
    void ctrlDragTearsOffAndAngleTurns();
    void ctrlClickCyclesAndDeleteRemoves();
    void showEdgesHidesOverlays();
    void undoRedoPlaneEdits();
    void gridSizeChangesTheGrid();
    void toolShortcuts();
    void cancelChangesNothing();
    void okRendersGridsAsOneState();
    void okWithoutRenderKeepsPlanesOnly();
    void renderIsClippedToTheSelection();
    void lastSettingsRestoresPlanes();
    void lastFilterRefusesOffEightBit();

private:
    // A white 400×300 8-bit RGB document, active.
    pictura::PictureView* fresh(const QString& name);
    // Show a dialog over a fresh document, exposed.
    std::unique_ptr<VanishingPointDialog> showDialog();
    // Run the menu command with `body` acting on the dialog it opens; false
    // when no dialog opened.
    bool runCommand(const QString& id, const std::function<void(VanishingPointDialog*)>& body);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void VanishingPointTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

pictura::PictureView* VanishingPointTest::fresh(const QString& name)
{
    if (!window_->newDocument(name, 400, 300, QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return nullptr;
    }
    return window_->activeView();
}

std::unique_ptr<VanishingPointDialog> VanishingPointTest::showDialog()
{
    if (!fresh(QStringLiteral("vp"))) {
        return nullptr;
    }
    auto dialog = std::make_unique<VanishingPointDialog>(window_->activeView(),
                                                         QStringLiteral("vp.png"));
    dialog->show();
    if (!QTest::qWaitForWindowExposed(dialog.get())) {
        return nullptr;
    }
    return dialog;
}

bool VanishingPointTest::runCommand(const QString& id,
                                    const std::function<void(VanishingPointDialog*)>& body)
{
    // A refused command never spins a modal loop, so the timer can fire after
    // this returns: it owns copies of everything it touches.
    auto opened = std::make_shared<bool>(false);
    auto timer = new QTimer(window_.get());
    timer->setSingleShot(true);
    connect(timer, &QTimer::timeout, timer, [opened, body, timer] {
        if (VanishingPointDialog* dialog = openVpDialog()) {
            *opened = true;
            body(dialog);
        }
        timer->deleteLater();
    });
    timer->start(0);
    window_->registry()->dispatch(id);
    // Drop a timer the refused path left armed, so it cannot act on a dialog
    // a later test opens.
    if (!*opened) {
        delete timer;
    }
    return *opened;
}

void VanishingPointTest::menuFollowsDepth()
{
    const pictura::FilterCommandSpec* spec = pictura::filterCommandForPath(kVpPath);
    QVERIFY(spec != nullptr);
    QCOMPARE(spec->kind, QStringLiteral("vanishing-point"));
    QCOMPARE(static_cast<int>(spec->dialog), static_cast<int>(pictura::FilterDialogKind::VanishingPoint));
    QCOMPARE(pictura::filter_param_arity(spec->kind), 1);
    QAction* action = window_->registry()->action(pictura::commandIdForPath(kVpPath));
    QVERIFY(action != nullptr);
    QCOMPARE(action->shortcut(), QKeySequence(QStringLiteral("Alt+Ctrl+V")));
    QVERIFY(fresh(QStringLiteral("Deep")));
    window_->registry()->refresh();
    QVERIFY(action->isEnabled());
    QVERIFY(window_->registry()->dispatch(QString::fromLatin1(pictura::command_ids::ImageMode16Bits)));
    QCOMPARE(pictura::image_depth_bits(*window_->activeView()), 16);
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    QVERIFY(window_->closeActiveDocument(false));
}

void VanishingPointTest::opensWithControls()
{
    auto dialog = showDialog();
    QVERIFY(dialog && dialog->ready());
    QVERIFY(dialog->windowTitle().startsWith(QStringLiteral("Vanishing Point (vp.png @ ")));
    QStringList tools;
    for (int i = 0; i < 4; ++i) {
        auto* button = dialog->findChild<QToolButton*>(QStringLiteral("vpTool%1").arg(i));
        QVERIFY(button != nullptr);
        tools << button->toolTip();
    }
    QCOMPARE(tools, QStringList({QStringLiteral("Edit Plane Tool (V)"),
                                 QStringLiteral("Create Plane Tool (C)"),
                                 QStringLiteral("Hand Tool (H)"), QStringLiteral("Zoom Tool (Z)")}));
    QVERIFY(dialog->findChild<QToolButton*>(QStringLiteral("vpTool4")) == nullptr);
    for (const QString& tip : tools) {
        QVERIFY(!tip.contains(QStringLiteral("Measure")));
    }
    auto* grid = dialog->findChild<QSpinBox*>(QStringLiteral("vpGridSize"));
    QCOMPARE(grid->value(), 100);
    QCOMPARE(grid->minimum(), 1);
    QCOMPARE(grid->maximum(), 1000);
    QVERIFY(!dialog->findChild<QSpinBox*>(QStringLiteral("vpAngle"))->isEnabled());
    QVERIFY(dialog->showEdgesAction()->isChecked());
    QVERIFY(!dialog->renderGridsAction()->isChecked());
    QCOMPARE(dialog->renderGridsAction()->text(), QStringLiteral("Render Grids To Photoshop"));
    QCOMPARE(dialog->tool(), int(VanishingPointDialog::CreatePlane));
    QCOMPARE(pictura::vp_plane_count(), 0);
    dialog->reject();
    QVERIFY(window_->closeActiveDocument(false));
}

void VanishingPointTest::fourClicksCreateAPlane()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    VpCanvas* canvas = dialog->canvas();
    dialog->selectTool(VanishingPointDialog::CreatePlane);
    for (int k = 0; k < 3; ++k) {
        click(canvas, kSquare.at(k));
    }
    QCOMPARE(canvas->pendingCorners(), 3);
    canvas->setFocus();
    QTest::keyClick(canvas, Qt::Key_Backspace);
    QCOMPARE(canvas->pendingCorners(), 2);
    click(canvas, kSquare.at(2));
    click(canvas, kSquare.at(3));
    QCOMPARE(canvas->pendingCorners(), 0);
    QCOMPARE(pictura::vp_plane_count(), 1);
    QCOMPARE(pictura::vp_plane_validity(0), 0);
    QCOMPARE(pictura::vp_active(), 0);
    QCOMPARE(dialog->tool(), int(VanishingPointDialog::EditPlane));
    QVERIFY(dialog->findChild<QSpinBox*>(QStringLiteral("vpGridSize"))->isEnabled());

    dialog->selectTool(VanishingPointDialog::CreatePlane);
    click(canvas, QPointF(20, 20));
    click(canvas, QPointF(60, 20));
    QCOMPARE(canvas->pendingCorners(), 2);
    QTest::keyClick(canvas, Qt::Key_Escape);
    QCOMPARE(canvas->pendingCorners(), 0);
    QCOMPARE(pictura::vp_plane_count(), 1);
    dialog->reject();
    QVERIFY(window_->closeActiveDocument(false));
}

void VanishingPointTest::dragReshapesAndMoves()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    makePlane(dialog.get(), kSquare);
    VpCanvas* canvas = dialog->canvas();
    drag(canvas, kSquare.at(2), QPointF(320, 250));
    QVERIFY(near(corner(0, 2), QPointF(320, 250)));
    const QPointF before[] = {corner(0, 0), corner(0, 1), corner(0, 2), corner(0, 3)};
    drag(canvas, QPointF(180, 120), QPointF(190, 130));
    for (int k = 0; k < 4; ++k) {
        QVERIFY2(near(corner(0, k), before[k] + QPointF(10, 10)), qPrintable(QString::number(k)));
    }
    dialog->reject();
    QVERIFY(window_->closeActiveDocument(false));
}

void VanishingPointTest::ctrlDragTearsOffAndAngleTurns()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    makePlane(dialog.get(), kSquare);
    VpCanvas* canvas = dialog->canvas();
    // The right edge's node is its middle: a square-on plane's metric
    // midpoint is the image midpoint.
    drag(canvas, QPointF(300, 150), QPointF(360, 150), Qt::ControlModifier);
    QCOMPARE(pictura::vp_plane_count(), 2);
    QCOMPARE(pictura::vp_active(), 1);
    QCOMPARE(pictura::vp_plane_validity(1), 0);
    auto* angle = dialog->findChild<QSpinBox*>(QStringLiteral("vpAngle"));
    QVERIFY(angle->isEnabled());
    QCOMPARE(angle->value(), 90);
    angle->setValue(45);
    QCOMPARE(pictura::vp_angle(), 45.0);
    click(canvas, QPointF(150, 150));
    QCOMPARE(pictura::vp_active(), 0);
    QVERIFY(!angle->isEnabled());
    dialog->reject();
    QVERIFY(window_->closeActiveDocument(false));
}

void VanishingPointTest::ctrlClickCyclesAndDeleteRemoves()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    makePlane(dialog.get(), kSquare);
    makePlane(dialog.get(), kOverlap);
    QCOMPARE(pictura::vp_plane_count(), 2);
    QCOMPARE(pictura::vp_active(), 1);
    VpCanvas* canvas = dialog->canvas();
    click(canvas, QPointF(250, 180), Qt::ControlModifier);
    QCOMPARE(pictura::vp_active(), 0);
    click(canvas, QPointF(250, 180), Qt::ControlModifier);
    QCOMPARE(pictura::vp_active(), 1);
    canvas->setFocus();
    QTest::keyClick(canvas, Qt::Key_Delete);
    QCOMPARE(pictura::vp_plane_count(), 1);
    QCOMPARE(pictura::vp_active(), -1);
    dialog->reject();
    QVERIFY(window_->closeActiveDocument(false));
}

void VanishingPointTest::showEdgesHidesOverlays()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    QVERIFY(dialog->canvas()->showEdges());
    QTest::keyClick(dialog.get(), Qt::Key_H, Qt::ControlModifier);
    QVERIFY(!dialog->showEdgesAction()->isChecked());
    QVERIFY(!dialog->canvas()->showEdges());
    QTest::keyClick(dialog.get(), Qt::Key_H, Qt::ControlModifier);
    QVERIFY(dialog->canvas()->showEdges());
    dialog->reject();
    QVERIFY(window_->closeActiveDocument(false));
}

void VanishingPointTest::undoRedoPlaneEdits()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    makePlane(dialog.get(), kSquare);
    QCOMPARE(pictura::vp_plane_count(), 1);
    QTest::keyClick(dialog.get(), Qt::Key_Z, Qt::ControlModifier);
    QCOMPARE(pictura::vp_plane_count(), 0);
    QTest::keyClick(dialog.get(), Qt::Key_Z, Qt::ControlModifier | Qt::ShiftModifier);
    QCOMPARE(pictura::vp_plane_count(), 1);
    dialog->reject();
    QVERIFY(window_->closeActiveDocument(false));
}

void VanishingPointTest::gridSizeChangesTheGrid()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    makePlane(dialog.get(), kSquare);
    const size_t coarse = pictura::vp_plane_grid_lines(0).size();
    dialog->findChild<QSpinBox*>(QStringLiteral("vpGridSize"))->setValue(50);
    QCOMPARE(pictura::vp_grid_size(), 50.0);
    QVERIFY(pictura::vp_plane_grid_lines(0).size() > coarse);
    dialog->reject();
    QVERIFY(window_->closeActiveDocument(false));
}

void VanishingPointTest::toolShortcuts()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    dialog->canvas()->setFocus();
    const struct {
        Qt::Key key;
        VanishingPointDialog::Tool tool;
    } keys[] = {{Qt::Key_V, VanishingPointDialog::EditPlane},
                {Qt::Key_H, VanishingPointDialog::Hand},
                {Qt::Key_Z, VanishingPointDialog::Zoom},
                {Qt::Key_C, VanishingPointDialog::CreatePlane}};
    for (const auto& k : keys) {
        QTest::keyClick(dialog.get(), k.key);
        QCOMPARE(dialog->tool(), int(k.tool));
    }
    dialog->reject();
    QVERIFY(window_->closeActiveDocument(false));
}

void VanishingPointTest::cancelChangesNothing()
{
    pictura::PictureView* view = fresh(QStringLiteral("cancel"));
    QVERIFY(view);
    const QImage document = view->image();
    const int history = view->history_count();
    const QString id = pictura::commandIdForPath(kVpPath);
    QVERIFY(runCommand(id, [](VanishingPointDialog* dialog) {
        makePlane(dialog, kSquare);
        dialog->renderGridsAction()->setChecked(true);
        dialog->reject();
    }));
    QCOMPARE(view->history_count(), history);
    QCOMPARE(view->image(), document);
    int planes = -1;
    QVERIFY(runCommand(id, [&planes](VanishingPointDialog* dialog) {
        planes = pictura::vp_plane_count();
        dialog->reject();
    }));
    QCOMPARE(planes, 0);
    QVERIFY(window_->closeActiveDocument(false));
}

void VanishingPointTest::okRendersGridsAsOneState()
{
    pictura::PictureView* view = fresh(QStringLiteral("render"));
    QVERIFY(view);
    const int history = view->history_count();
    QVERIFY(runCommand(pictura::commandIdForPath(kVpPath), [](VanishingPointDialog* dialog) {
        makePlane(dialog, kSquare);
        dialog->renderGridsAction()->setChecked(true);
        dialog->accept();
    }));
    QCOMPARE(view->history_count(), history + 1);
    QVERIFY(view->history_label(view->history_count() - 1).contains(QStringLiteral("Vanishing Point")));
    QCOMPARE(filter_last_kind(*view), QStringLiteral("vanishing-point"));
    QCOMPARE(filter_last_params(*view).size(), 1);
    const QImage image = view->image();
    // A 200×180 square-on plane measures its pixels: the outline at x = 100
    // and the grid line at x = 200 straddle pixel boundaries.
    QVERIFY(!white(image, 99, 150) && !white(image, 100, 150));
    QVERIFY(!white(image, 199, 150) && !white(image, 200, 150));
    QVERIFY(white(image, 150, 150));
    QVERIFY(white(image, 50, 20) && white(image, 350, 280));
    QVERIFY(window_->closeActiveDocument(false));
}

void VanishingPointTest::okWithoutRenderKeepsPlanesOnly()
{
    pictura::PictureView* view = fresh(QStringLiteral("planes"));
    QVERIFY(view);
    const QImage document = view->image();
    const int history = view->history_count();
    const QString id = pictura::commandIdForPath(kVpPath);
    // Nothing to commit, so the command reports no dialog result.
    runCommand(id, [](VanishingPointDialog* dialog) {
        makePlane(dialog, kSquare);
        dialog->accept();
    });
    QCOMPARE(view->history_count(), history);
    QCOMPARE(view->image(), document);
    int planes = -1;
    bool render = true;
    int tool = -1;
    QVERIFY(runCommand(id, [&](VanishingPointDialog* dialog) {
        planes = pictura::vp_plane_count();
        render = dialog->renderGridsAction()->isChecked();
        tool = dialog->tool();
        dialog->reject();
    }));
    QCOMPARE(planes, 1);
    QVERIFY(!render);
    QCOMPARE(tool, int(VanishingPointDialog::EditPlane));
    QVERIFY(fresh(QStringLiteral("other")));
    planes = -1;
    QVERIFY(runCommand(id, [&planes](VanishingPointDialog* dialog) {
        planes = pictura::vp_plane_count();
        dialog->reject();
    }));
    QCOMPARE(planes, 0);
    QVERIFY(window_->closeActiveDocument(false));
    QVERIFY(window_->closeActiveDocument(false));
}

void VanishingPointTest::renderIsClippedToTheSelection()
{
    pictura::PictureView* view = fresh(QStringLiteral("clip"));
    QVERIFY(view);
    QVERIFY(view->select_rect(0, 0, 200, 300, QStringLiteral("new"), 0.0));
    QVERIFY(runCommand(pictura::commandIdForPath(kVpPath), [](VanishingPointDialog* dialog) {
        makePlane(dialog, kSquare);
        dialog->renderGridsAction()->setChecked(true);
        dialog->accept();
    }));
    const QImage image = view->image();
    // Inside the selection (x < 200) the outline and grid line are drawn;
    // past it the grid line's other half and the right outline are not.
    QVERIFY(!white(image, 100, 150) && !white(image, 199, 150));
    QVERIFY(!white(image, 150, 60));
    QVERIFY(white(image, 200, 150) && white(image, 299, 150) && white(image, 300, 150));
    QVERIFY(white(image, 250, 60));
    QVERIFY(window_->closeActiveDocument(false));
}

void VanishingPointTest::lastSettingsRestoresPlanes()
{
    pictura::PictureView* view = fresh(QStringLiteral("last"));
    QVERIFY(view);
    const QString id = pictura::commandIdForPath(kVpPath);
    QVERIFY(runCommand(id, [](VanishingPointDialog* dialog) {
        makePlane(dialog, kSquare);
        makePlane(dialog, {QPointF(320, 40), QPointF(390, 40), QPointF(390, 120),
                           QPointF(320, 120)});
        dialog->renderGridsAction()->setChecked(true);
        dialog->accept();
    }));
    QCOMPARE(filter_last_kind(*view), QStringLiteral("vanishing-point"));
    runCommand(id, [](VanishingPointDialog* dialog) {
        while (pictura::vp_plane_count() > 0) {
            pictura::vp_set_active(0);
            pictura::vp_delete_active();
        }
        dialog->accept();
    });
    int planes = -1;
    bool render = false;
    QVERIFY(runCommand(QString::fromLatin1(pictura::command_ids::FilterLastFilterSettings),
                       [&](VanishingPointDialog* dialog) {
                           planes = pictura::vp_plane_count();
                           render = dialog->renderGridsAction()->isChecked();
                           dialog->reject();
                       }));
    QCOMPARE(planes, 2);
    QVERIFY(render);
    QVERIFY(window_->closeActiveDocument(false));
}

void VanishingPointTest::lastFilterRefusesOffEightBit()
{
    pictura::PictureView* view = fresh(QStringLiteral("deep"));
    QVERIFY(view);
    QVERIFY(runCommand(pictura::commandIdForPath(kVpPath), [](VanishingPointDialog* dialog) {
        makePlane(dialog, kSquare);
        dialog->renderGridsAction()->setChecked(true);
        dialog->accept();
    }));
    QCOMPARE(filter_last_kind(*view), QStringLiteral("vanishing-point"));
    QVERIFY(window_->registry()->dispatch(QString::fromLatin1(pictura::command_ids::ImageMode16Bits)));
    QCOMPARE(pictura::image_depth_bits(*view), 16);
    const int history = view->history_count();
    QVERIFY(!runCommand(QString::fromLatin1(pictura::command_ids::FilterLastFilterSettings),
                        [](VanishingPointDialog* dialog) { dialog->accept(); }));
    window_->registry()->dispatch(QString::fromLatin1(pictura::command_ids::FilterLastFilter));
    QCOMPARE(view->history_count(), history);
    QVERIFY(filter_last_error(*view).contains(QStringLiteral("8-bit")));
    QVERIFY(window_->closeActiveDocument(false));
}

QTEST_MAIN(VanishingPointTest)
#include "tst_vanishing_point.moc"
